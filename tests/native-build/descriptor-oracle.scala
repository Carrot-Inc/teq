//> using dep org.scala-native::tools:0.5.12
// The JVM's answers of tests/native-build/descriptors.expected: each descriptor of tests/native-build/descriptor-features.txt
// (each piece alone, and with every piece after it) written to a file and read by Scala Native 0.5.12's own
// `Descriptor.load` (a FileReader, Properties.load, parseStrings, toBoolean), with what `Properties.load` gives of
// the file and the include paths as `NativeLib.createPathString` resolves them under /code. It calls Scala Native's
// own class, private to its package (hence the package), so it runs on a JVM, never under teq: on a JDK of 18 or
// later (whose default charset is UTF-8), from the repository's root, once for each Scala of Scala Native's builds,
// then merged:
//   scala-cli run --scala 2.12.20 tests/native-build/descriptor-oracle.scala -- write <features> <a>
//   scala-cli run --scala 3.3.6 tests/native-build/descriptor-oracle.scala -- write <features> <b>
//   scala-cli run --scala 3.3.6 tests/native-build/descriptor-oracle.scala -- merge <a> <b> tests/native-build/descriptors.expected
// The two differ only where `toBoolean` does (2.12's lowers the string, 2.13's compares ignoring case); there the
// merged answer of `gc` is `error`, as neither build is followed.
package scala.scalanative.build

import java.nio.charset.{Charset, StandardCharsets}
import java.nio.file.{Files, Path, Paths}
import scala.collection.JavaConverters._
import scala.util.{Failure, Success, Try}

object DescriptorOracle {
  def quoted(s: String): String =
    "\"" + s.map(c => if (c >= 0x20 && c < 0x7f && c != '"' && c != '\\') c.toString else "\\u%04X".format(c.toInt)).mkString + "\""

  def list(name: String, items: Seq[String]): String =
    if (items.isEmpty) "" else s" $name=[" + items.map(quoted).mkString(",") + "]"

  def unescaped(text: String): Array[Byte] = {
    val out = new java.io.ByteArrayOutputStream
    var at = 0
    while (at < text.length) {
      if (text.charAt(at) == '\\' && at + 1 < text.length && text.charAt(at + 1) == 'x') {
        out.write(Integer.parseInt(text.substring(at + 2, at + 4), 16))
        at += 4
      } else if (text.charAt(at) == '\\' && at + 1 < text.length && text.charAt(at + 1) == '\\') {
        out.write('\\')
        at += 2
      } else {
        out.write(text.charAt(at).toInt)
        at += 1
      }
    }
    out.toByteArray
  }

  /** NativeLib.createPathString, which is private to its object: the same lines. */
  def createPathString(unixPath: String, nativeCodePath: Path): String =
    unixPath.split("/").foldLeft(nativeCodePath)((path, dir) => path.resolve(dir)).toAbsolutePath.toString

  /** A descriptor's answer in three fields, `gc`, what `Properties.load` gives and the rest, for `merge` to join. */
  def answer(file: Path): String = {
    val props = new java.util.Properties
    val loaded = Try {
      val reader = new java.io.FileReader(file.toFile)
      try props.load(reader)
      finally reader.close()
    }
    loaded match {
      case Failure(e) => "-\terror " + quoted(e.getMessage) + "\t"
      case Success(_) =>
        val sorted = props.stringPropertyNames.asScala.toSeq.sorted
        val properties = "{" + sorted.map(k => quoted(k) + ":" + quoted(props.getProperty(k))).mkString(",") + "}"
        Descriptor.load(file) match {
          // Of what Properties.load takes, Descriptor.load throws for project.gcProject alone.
          case Failure(_) => "error\t" + properties + "\t" + rest(props, None)
          case Success(d) => d.gcProject.toString + "\t" + properties + "\t" + rest(props, Some(d))
        }
    }
  }

  /** The lists, from the descriptor where it loads, else as `parseStrings` reads them. */
  def rest(props: java.util.Properties, d: Option[Descriptor]): String = {
    def strings(key: String): Seq[String] = Option(props.getProperty(key)).map(_.split(',').map(_.trim()).toSeq).getOrElse(Seq.empty)
    val links = d.map(_.links).getOrElse(strings("nir.link.names"))
    val defines = d.map(_.defines).getOrElse(strings("preprocessor.defines"))
    val includes = d.map(_.includes).getOrElse(strings("compile.include.paths"))
    val c = d.map(_.cOptions).getOrElse(strings("compile.c.options"))
    val cpp = d.map(_.cppOptions).getOrElse(strings("compile.cpp.options"))
    val paths = Try(includes.map(createPathString(_, Paths.get("/code")))) match {
      case Success(p) => list("paths", p)
      case Failure(_) => " paths=error"
    }
    list("links", links) + list("defines", defines) + list("includes", includes) + paths + list("c", c) + list("cpp", cpp)
  }

  def write(featuresFile: Path, out: Path): Unit = {
    require(Charset.defaultCharset == StandardCharsets.UTF_8, s"the default charset is ${Charset.defaultCharset}, not UTF-8: run on a JDK of 18 or later")
    val features = Files.readAllLines(featuresFile, StandardCharsets.UTF_8).asScala.toVector.filter(l => l.nonEmpty && !l.startsWith("#")).map { line =>
      val at = line.indexOf(' ')
      if (at < 0) (line, Array.emptyByteArray) else (line.substring(0, at), unescaped(line.substring(at + 1)))
    }
    val descriptors = features ++ (for (i <- features.indices; j <- i + 1 until features.length)
      yield (features(i)._1 + "+" + features(j)._1, features(i)._2 ++ features(j)._2))
    val dir = Files.createTempDirectory("descriptor-oracle")
    val lines = descriptors.map { case (id, bytes) =>
      val file = dir.resolve("scala-native.properties")
      Files.write(file, bytes)
      s"$id\t${answer(file)}"
    }
    val header = s"# Scala ${scala.util.Properties.versionNumberString}, Java ${System.getProperty("java.version")} ${System.getProperty("java.vendor")}"
    Files.write(out, (header +: lines).mkString("", "\n", "\n").getBytes(StandardCharsets.UTF_8))
  }

  /** The two builds' answers as one: the same but for `gc`, which is `error` where they differ. */
  def merge(a: Path, b: Path, out: Path): Unit = {
    def rows(p: Path) = Files.readAllLines(p, StandardCharsets.UTF_8).asScala.toVector
    val (ra, rb) = (rows(a), rows(b))
    require(ra.length == rb.length && ra.length > 1, s"$a and $b do not hold the same descriptors")
    val header = Vector(
      "# What Scala Native 0.5.12's Descriptor.load reads of each descriptor of tests/native-build/descriptor-features.txt,",
      "# written by tests/native-build/descriptor-oracle.scala (which says how to run it again) under:",
      ra.head, rb.head)
    val merged = ra.tail.zip(rb.tail).map { case (x, y) =>
      val (Array(id, gcA, props, rest), Array(idB, gcB, propsB, restB)) = (x.split("\t", -1), y.split("\t", -1))
      require(id == idB && props == propsB && rest == restB, s"$x and $y differ in more than gc")
      val gc = if (gcA != gcB) " gc=error" else gcA match {
        case "true" => " gc=true"
        case "error" => " gc=error"
        case _ => ""
      }
      s"$id $props$gc$rest"
    }
    Files.write(out, (header ++ merged).mkString("", "\n", "\n").getBytes(StandardCharsets.UTF_8))
  }

  def main(args: Array[String]): Unit = args.toList match {
    case List("write", features, out) => write(Paths.get(features), Paths.get(out))
    case List("merge", a, b, out) => merge(Paths.get(a), Paths.get(b), Paths.get(out))
    case _ => sys.error("descriptor-oracle.scala write <features> <out> | merge <a> <b> <out>")
  }
}

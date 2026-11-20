//> using scala 3.8.4
//> using dep org.scala-lang::scala3-compiler:3.8.4
// Batch.scala <list file> <out root>: each listed program directory compiled by dotty 3.8.4 in this one JVM (its own
// output directory, its own reporter), then, where it compiles, its `use.run` main run in its own class loader with
// the output captured. Per program <out root>/<name>.exit (0, or 1 when refused, or 2 when it dies), .out, .err.
import java.io.*
import java.nio.file.*
import scala.jdk.CollectionConverters.*

@main def batch(listFile: String, outRoot: String): Unit =
  val root = Paths.get(outRoot); Files.createDirectories(root)
  for dir <- Files.readAllLines(Paths.get(listFile)).asScala if dir.trim.nonEmpty do
    val name = Paths.get(dir).getFileName.toString
    if !Files.exists(root.resolve(name + ".exit")) then
      val classes = Files.createDirectories(root.resolve("classes").resolve(name))
      val srcs = Files.list(Paths.get(dir)).iterator.asScala.map(_.toString).filter(_.endsWith(".scala")).toArray.sorted
      val err = new ByteArrayOutputStream(); val out = new ByteArrayOutputStream()
      val errPs = new PrintStream(err, true, "UTF-8"); val outPs = new PrintStream(out, true, "UTF-8")
      val ok = scala.Console.withErr(errPs) {
        val rep = dotty.tools.dotc.Main.process(Array("-usejavacp", "-color:never", "-d", classes.toString) ++ srcs)
        !rep.hasErrors
      }
      var code = if ok then 0 else 1
      if ok then
        val loader = new java.net.URLClassLoader(Array(classes.toUri.toURL), getClass.getClassLoader)
        try
          val mains = Files.walk(classes).iterator.asScala.map(classes.relativize(_).toString)
            .filter(f => f.endsWith(".class") && !f.contains("$")).map(_.stripSuffix(".class").replace('/', '.')).toList.sorted
            .map(loader.loadClass).filter(c => scala.util.Try(c.getMethod("main", classOf[Array[String]])).toOption
              .exists(m => java.lang.reflect.Modifier.isStatic(m.getModifiers)))
          val cls = mains.find(_.getName == "use.run").orElse(mains.headOption).getOrElse(sys.error("no main"))
          scala.Console.withOut(outPs) { scala.Console.withErr(errPs) {
            cls.getMethod("main", classOf[Array[String]]).invoke(null, Array.empty[String])
          } }
        catch case e: Throwable =>
          val c = e match { case i: java.lang.reflect.InvocationTargetException => i.getCause; case o => o }
          errPs.println(s"Exception in thread \"main\" $c"); code = 2
        finally loader.close()
      outPs.flush(); errPs.flush()
      Files.write(root.resolve(name + ".out"), out.toByteArray)
      Files.write(root.resolve(name + ".err"), err.toByteArray)
      Files.writeString(root.resolve(name + ".exit"), s"$code\n")

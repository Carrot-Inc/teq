package dev.teq.sbt

import scala.collection.mutable
import scala.collection.compat.*

/** The scalac options a build gives, mapped onto teq's flags (docs/TOOLING.md, "The lockfile and the
  * launchers", the flags): what teq honours, each by its field, and what it ignores, by its words; the
  * export of each build tool writes them alike (sbt-teq's `Export`, mill-teq's). */
private[teq] final case class ScalacOptions(
  maxInlines: Option[Int],
  strictEquality: Boolean,
  kindProjector: Boolean,
  werror: Boolean,
  javaOutputVersion: Option[Int],
  ignored: Seq[String],
  wunused: Seq[String] = Nil,
  deprecation: Boolean = false,
  feature: Boolean = false,
  wtostringInterpolated: Boolean = false,
  wconf: Seq[String] = Nil,
  language: Seq[String] = Nil,
  source: Option[String] = None,
) {
  /** Whether the unused imports are reported, as scalac's `WunusedHas.imports` reads the kinds. */
  def wunusedImports: Boolean =
    (wunused.contains("all") || wunused.contains("imports") || wunused.contains("linted")) && !wunused.contains("strict-no-implicit-warn")

  /** The teq flags the options map onto; the output version is a JVM build's alone. */
  def flags(jvm: Boolean): Seq[String] =
    maxInlines.toSeq.flatMap(n => Seq("--max-inlines", n.toString)) ++
      (if (strictEquality) Seq("--strict-equality") else Nil) ++
      (if (kindProjector) Seq("--kind-projector") else Nil) ++
      (if (language.nonEmpty) Seq("--language", language.mkString(",")) else Nil) ++
      (if (werror) Seq("--werror") else Nil) ++
      (if (wunused.nonEmpty) Seq("--wunused", wunused.mkString(",")) else Nil) ++
      (if (deprecation) Seq("--deprecation") else Nil) ++
      (if (feature) Seq("--feature") else Nil) ++
      (if (wtostringInterpolated) Seq("--wtostring-interpolated") else Nil) ++
      wconf.flatMap(rules => Seq("--wconf", rules)) ++
      source.toSeq.flatMap(v => Seq("--source", v)) ++
      (if (jvm) javaOutputVersion.toSeq.flatMap(n => Seq("--java-output-version", n.toString)) else Nil)
}

private[teq] object ScalacOptions {
  private val withArgument = Set("-release", "-encoding", "-java-output-version")
  /** scalac's `-source` and its alias, whose value teq's `--source` takes as it is, refusing what scalac refuses. */
  private val sourceNames = Seq("-source", "--source")
  /** `-release` names the class files' version too, unless `-java-output-version` does. */
  private val outputVersion = Seq("-java-output-version", "-release")

  def apply(options: Seq[String]): ScalacOptions = {
    var maxInlines = Option.empty[Int]
    var strictEquality = false
    var kindProjector = false
    var werror = false
    var deprecation = false
    var feature = false
    var wtostringInterpolated = false
    val wunused = mutable.ArrayBuffer.empty[String]
    val wconf = mutable.ArrayBuffer.empty[String]
    val language = mutable.ArrayBuffer.empty[String]
    var source = Option.empty[String]
    val versions = mutable.Map.empty[String, Int]
    val ignored = mutable.ArrayBuffer.empty[String]
    var rest = options.toList
    while (rest.nonEmpty)
      rest match {
        case option :: n :: tail if outputVersion.contains(option) && n.toIntOption.isDefined =>
          versions(option) = n.toInt
          if (option == "-release") ignored += s"$option $n"
          rest = tail
        case option :: tail if outputVersion.exists(name => option.startsWith(name + ":")) && option.dropWhile(_ != ':').drop(1).toIntOption.isDefined =>
          val name = option.takeWhile(_ != ':')
          versions(name) = option.dropWhile(_ != ':').drop(1).toInt
          if (name == "-release") ignored += option
          rest = tail
        case "-Xmax-inlines" :: n :: tail if n.toIntOption.isDefined =>
          maxInlines = n.toIntOption
          rest = tail
        case option :: tail if option.startsWith("-Xmax-inlines:") && option.stripPrefix("-Xmax-inlines:").toIntOption.isDefined =>
          maxInlines = option.stripPrefix("-Xmax-inlines:").toIntOption
          rest = tail
        // scalac's source version, in either form; the last given is the setting, as scalac's driver reads it.
        case option :: v :: tail if sourceNames.contains(option) =>
          source = Some(v)
          rest = tail
        case option :: tail if sourceNames.exists(name => option.startsWith(name + ":")) =>
          source = Some(option.dropWhile(_ != ':').drop(1))
          rest = tail
        case "-Xkind-projector" :: tail =>
          kindProjector = true
          rest = tail
        case ("-Werror" | "-Xfatal-warnings") :: tail =>
          werror = true
          rest = tail
        case ("-deprecation" | "--deprecation") :: tail =>
          deprecation = true
          rest = tail
        case ("-feature" | "--feature") :: tail =>
          feature = true
          rest = tail
        case "-Wtostring-interpolated" :: tail =>
          wtostringInterpolated = true
          rest = tail
        // scalac's `-Wconf` rules, in their order: teq's `--wconf`, the rightmost deciding.
        case option :: tail if option.startsWith("-Wconf:") =>
          wconf += option.stripPrefix("-Wconf:")
          rest = tail
        // scalac's `-Wunused` with its kinds, each given adding to the ones before, which teq
        // takes as they are (`--wunused`); a bare `-Wunused` is every kind.
        case "-Wunused" :: tail =>
          wunused += "all"
          rest = tail
        case option :: tail if option.startsWith("-Wunused:") =>
          wunused ++= option.stripPrefix("-Wunused:").split(",").toSeq.map(_.trim).filter(_.nonEmpty)
          rest = tail
        // `-Wall` asks for every unused kind and `-Wtostring-interpolated` among the rest of its
        // warnings, which teq does not give.
        case "-Wall" :: tail =>
          wunused += "all"
          wtostringInterpolated = true
          ignored += "-Wall"
          rest = tail
        case option :: tail if option.startsWith("-language:") =>
          val (strict, others) = option.stripPrefix("-language:").split(",").toSeq.map(_.trim).filter(_.nonEmpty).partition(_ == "strictEquality")
          if (strict.nonEmpty) strictEquality = true
          language ++= others
          rest = tail
        case option :: argument :: tail if withArgument(option) =>
          ignored += s"$option $argument"
          rest = tail
        case option :: tail =>
          ignored += option
          rest = tail
        case Nil => ()
      }
    val javaOutputVersion = outputVersion.collectFirst { case name if versions.contains(name) => versions(name) }
    ScalacOptions(maxInlines, strictEquality, kindProjector, werror, javaOutputVersion, ignored.distinct.toSeq, wunused.distinct.toSeq, deprecation, feature, wtostringInterpolated, wconf.toSeq, language.distinct.toSeq, source)
  }
}

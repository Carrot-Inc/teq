import sbt.*
import sbt.Keys.*
import dev.teq.sbt.TeqPlugin
import dev.teq.sbt.TeqPlugin.autoImport.*

// lila's generators as teq runs them, in the place of the sbt tasks that write the same files
// (sbt-teq's `teqReplacesSbtGenerators`): liplay's routers, by its routes compiler under
// `teq interp` (bench/lila/routes.scala), the forward routers of the root project `lila` and the
// reverse ones of `ui`, over the files lila's build.sbt names (conf/routes and conf/*.routes),
// their Test configurations generating nothing (no routes file among their resources), which the
// setting records; and i18n's serialised translations (bench/lila/i18n.scala, project/I18n.scala's
// resource generator, over the databases build.sbt lists) into a resource directory of the
// project's. The routes compiler's jars are the ones this meta-build loaded liplay's plugin with.
object LilaTeq extends AutoPlugin:
  override def requires = TeqPlugin
  override def trigger = allRequirements

  private def scripts: File = file(sys.props.getOrElse("teq.lila.scripts", sys.error("teq.lila.scripts names no directory")))
  private def jarOf(className: String): String =
    new File(Class.forName(className).getProtectionDomain.getCodeSource.getLocation.toURI).getAbsolutePath
  private def routesClasspath: String =
    Seq("play.routes.compiler.RoutesCompiler", "play.twirl.api.Txt", "scala.util.parsing.combinator.Parsers", "scala.xml.Node")
      .map(jarOf).distinct.mkString(java.io.File.pathSeparator)

  private def routes(root: File, forward: Boolean, reverse: Boolean): TeqCommand =
    val conf = root / "conf"
    val files = ((conf * "routes").get() ++ (conf * "*.routes").get()).map(f => IO.relativize(root, f).get).sorted
    TeqCommand(
      Seq("teq", "interp", (scripts / "routes.scala").getAbsolutePath, "--classpath", routesClasspath, "--", forward.toString, reverse.toString) ++ files,
      inputs = Seq("conf/routes", "conf/*.routes"))

  // build.sbt's `i18n` resource generator's databases, in its order.
  private val databases = "activity app appeal arena broadcast challenge class coach contact coordinates dgt emails faq features insight keyboardMove lag learn nvui oauthScope onboarding patron perfStat practice preferences puzzle puzzleTheme recap search settings site streamer storm study swiss team timeago tfa tourname ublog variant video voiceCommands msg".split(' ').toSeq
  private val translations = "target/teq/i18n/resources"

  private def i18n: TeqCommand =
    TeqCommand(
      Seq("teq", "interp", (scripts / "i18n.scala").getAbsolutePath, "--", "translation/source", "translation/dest", translations, databases.mkString(",")),
      inputs = Seq("translation/source/*.xml", "translation/dest/**/*.xml"),
      outputs = Seq(translations))

  override def projectSettings = Seq(
    Compile / teqGenerators ++= (thisProject.value.id match
      case "lila" => Seq(routes((LocalRootProject / baseDirectory).value, forward = true, reverse = false))
      case "ui" => Seq(routes((LocalRootProject / baseDirectory).value, forward = false, reverse = true))
      case "i18n" => Seq(i18n)
      case _ => Nil),
    Compile / unmanagedResourceDirectories ++= (if thisProject.value.id == "i18n" then Seq((LocalRootProject / baseDirectory).value / translations) else Nil),
    Compile / teqReplacesSbtGenerators := Set("lila", "ui", "i18n").contains(thisProject.value.id),
    Test / teqReplacesSbtGenerators := Set("lila", "ui", "i18n").contains(thisProject.value.id),
  )

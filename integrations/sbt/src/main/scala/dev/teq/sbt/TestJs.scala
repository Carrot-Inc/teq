package dev.teq.sbt

import java.io.File
import scala.collection.compat.*
import scala.util.control.NonFatal

/** How sbt-scalajs runs a Scala.js project's `Test` configuration, for teqExportAll: the test
  * configuration's `moduleKind` and `jsEnv` of teq.lock (`Export.teqExportScalaJSTest`), and the reasons
  * teq cannot run it, which the lock records as refusals of `test` (`Export.teqExportScalaJSTestRefusals`).
  * A JVM project has neither. */
final case class TestJs(fields: Map[String, Json.Value], refusals: Seq[String])

object TestJs {
  val none: TestJs = TestJs(Map.empty, Nil)

  val NodeJSEnv = "org.scalajs.jsenv.nodejs.NodeJSEnv"
  val JSDOMNodeJSEnv = "org.scalajs.jsenv.jsdomnodejs.JSDOMNodeJSEnv"
  private val Lockfiles = Seq("package-lock.json", "npm-shrinkwrap.json", "yarn.lock", "pnpm-lock.yaml")

  /** The refusals of what sbt-scalajs loads before the test link besides the link itself: its
    * `Test / jsEnvInput` is the project's own `jsEnvInput` (the Zero configuration's, `Nil` unless
    * the build adds to it) followed by the link's main module, which teq's run supplies itself.
    * The inputs the build adds at the project's level (`project`, described) and its own
    * definitions of `Test / jsEnvInput` beside sbt-scalajs's two (`test`, where they stand), whose
    * inputs only linking would tell, load before the tests what teq's run does not. */
  def inputRefusals(project: Seq[String], test: Seq[String]): Seq[String] = {
    val own = if (project.isEmpty) Nil else Seq(s"jsEnvInput loads ${project.mkString(", ")} before the test link, which teq's run does not")
    own ++ test.map(at => s"Test / jsEnvInput is set at $at beside sbt-scalajs's input of the link: what it adds loads before the test link, which teq's run does not")
  }

  /** Whether a setting stands in sbt-scalajs's own `ScalaJSPluginInternal.scala`, whose two
    * definitions of `Test / jsEnvInput` are the link's: sbt 2 gives the position's file by its name,
    * sbt 1 after the definition's owner
    * (`(org.scalajs.sbtplugin.ScalaJSPluginInternal.scalaJSConfigSettings) ScalaJSPluginInternal.scala`). */
  def fromScalaJS(path: String): Boolean = path.endsWith("ScalaJSPluginInternal.scala")

  /** The fields of a link of the module kind under `env` (`Test / jsEnv`), the refusals found so far
    * added to: `jsEnv` as `TeqScalaJSPlugin.jsEnvJson` records any environment, and what teq's run
    * of a `NodeJSEnv` or a `JSDOMNodeJSEnv` takes besides, read from its field `config`
    * (scalajs-env-nodejs 1.4 to 1.6, scalajs-env-jsdom-nodejs 1.1) through the classes' names, the
    * jsdom one being a library of the build's and not of the plugin's: `NodeJSEnv`'s source-map
    * policy (`sourceMap`), and the package root jsdom is required from, the build's root where node
    * runs under sbt, with the lockfile there that installs it (`packageRoot`, `lockfile`, none where
    * the root holds none). A node or jsdom environment whose configuration does not read is a
    * refusal; `teq test` refuses another environment, and a jsdom without a lockfile, by `jsEnv`. */
  def of(moduleKind: String, env: AnyRef, root: File, refusals: Seq[String]): TestJs = {
    def field(of: AnyRef, name: String): Option[AnyRef] =
      try {
        val f = of.getClass.getDeclaredField(name)
        f.setAccessible(true)
        Option(f.get(of))
      } catch { case NonFatal(_) => None }
    def call(on: AnyRef, name: String): Option[Any] =
      try Some(on.getClass.getMethod(name).invoke(on)) catch { case NonFatal(_) => None }
    val envClass = env.getClass.getName
    val jsdom = envClass == JSDOMNodeJSEnv
    val config = field(env, "config")
    val readable = Seq("executable", "args", "env").forall(name => config.flatMap(call(_, name)).isDefined)
    val more =
      if (jsdom) {
        val lockfile = Lockfiles.find(n => new File(root, n).isFile)
        Map("packageRoot" -> Json.Str(".")) ++ lockfile.map(l => "lockfile" -> Json.Str(l))
      } else if (envClass == NodeJSEnv) {
        val policy = config.flatMap(call(_, "sourceMap")).map(_.toString) match {
          case Some("Disable") => "disable"
          case Some("Enable") => "enable"
          case _ => "enableIfAvailable"
        }
        Map("sourceMap" -> Json.Str(policy))
      } else Map.empty[String, Json.Value]
    val jsEnv = TeqScalaJSPlugin.jsEnvJson(env) match {
      case Json.Obj(own) => Json.Obj(own ++ more)
      case other => other
    }
    val unread = Option.when((jsdom || envClass == NodeJSEnv) && !readable)(
      s"Test / jsEnv is a $envClass whose configuration the export does not read (its field config, as scalajs-env-nodejs 1.4 to 1.6 and scalajs-env-jsdom-nodejs 1.1 keep it)")
    TestJs(Map("moduleKind" -> Json.Str(moduleKind), "jsEnv" -> jsEnv), refusals ++ unread)
  }
}

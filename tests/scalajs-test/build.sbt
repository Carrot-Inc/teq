// The Scala.js test fixture of `teq test` (tests/task.sh), run by sbt-scalajs as the oracle
// (tests/scalajs-oracle.scala), which records every event of every test under the tuple of
// versions the system properties name: Scala.js 1.22.0 with munit 1.3.4, ScalaTest 3.2.20,
// zio-test 2.1.26 and utest 0.9.1 by default; Scala.js 1.20.1 with munit 1.2.1 and without
// zio-test, which needs 1.21, under `-Dfixture.scalajs=1.20.1 -Dfixture.munit=1.2.1
// -Dfixture.zio=false`. `suites` runs under node with no module, `dom` under jsdom (from this
// directory's node_modules, package-lock.json's), `esm` as an ES module importing node's `path`,
// `cjs` as a CommonJS module under node with an argument and a variable.
import org.scalajs.jsenv.jsdomnodejs.JSDOMNodeJSEnv
import org.scalajs.jsenv.nodejs.NodeJSEnv
import org.scalajs.linker.interface.ModuleKind

ThisBuild / scalaVersion := "3.8.4"

val munitVersion = sys.props.getOrElse("fixture.munit", "1.3.4")
val zio = sys.props.getOrElse("fixture.zio", "true") == "true"

// Each event as a line of the project's oracle-events.txt: the suite, the selector, the status and
// the first line of the throwable's toString.
def recorder(project: String, file: File): TestReportListener = new TestReportListener {
  def doInit(): Unit = ()
  def startGroup(name: String): Unit = ()
  def testEvent(event: TestEvent): Unit = event.detail.foreach { e =>
    val selector = e.selector match {
      case s: sbt.testing.TestSelector => "test " + s.testName
      case s: sbt.testing.NestedTestSelector => "nested " + s.suiteId + " " + s.testName
      case s: sbt.testing.NestedSuiteSelector => "nestedSuite " + s.suiteId
      case s: sbt.testing.TestWildcardSelector => "wildcard " + s.testWildcard
      case _ => "suite"
    }
    val thrown = if (e.throwable.isDefined) e.throwable.get.toString.linesIterator.toList.headOption.getOrElse("") else ""
    IO.append(file, s"$project\t${e.fullyQualifiedName}\t$selector\t${e.status}\t$thrown\n")
  }
  def endGroup(name: String, result: TestResult): Unit = ()
  def endGroup(name: String, t: Throwable): Unit = IO.append(file, s"$project\t$name\tsuite\tAborted\t$t\n")
  def doComplete(result: TestResult): Unit = ()
}

// The suites sbt's discovery defines, a line each in the same file.
val oracleDefined = taskKey[Unit]("Records the suites sbt's discovery defines")

def fixture(p: Project): Project = p
  .enablePlugins(ScalaJSPlugin)
  .settings(
    Test / testListeners += recorder(thisProject.value.id, (ThisBuild / baseDirectory).value / "target" / "oracle-events.txt"),
    oracleDefined := {
      val file = (ThisBuild / baseDirectory).value / "target" / "oracle-events.txt"
      (Test / definedTests).value.foreach(d => IO.append(file, s"${thisProject.value.id}\t${d.name}\tdefined\t\t\n"))
    },
    libraryDependencies += "org.scalameta" %%% "munit" % munitVersion % Test,
  )

lazy val suites = project
  .configure(fixture)
  .settings(
    libraryDependencies ++= Seq(
      "org.scalatest" %%% "scalatest" % "3.2.20" % Test,
      "com.lihaoyi" %%% "utest" % "0.9.1" % Test,
    ) ++ (if (zio) Seq("dev.zio" %%% "zio-test" % "2.1.26" % Test, "dev.zio" %%% "zio-test-sbt" % "2.1.26" % Test) else Nil),
    testFrameworks += new TestFramework("utest.runner.Framework"),
    Test / unmanagedSourceDirectories ++= (if (zio) Seq(baseDirectory.value / "src" / "test" / "zio") else Nil),
    Test / testOptions += Tests.Exclude(Seq("fixture.MunitExcluded")),
    Test / testOptions += Tests.Argument(TestFrameworks.MUnit, "--exclude-tags=Slow"),
  )

lazy val dom = project
  .configure(fixture)
  .settings(
    libraryDependencies ++= Seq(
      "org.scalatest" %%% "scalatest" % "3.2.20" % Test,
      "org.scala-js" %%% "scalajs-dom" % "2.8.1" % Test,
    ),
    Test / jsEnv := new JSDOMNodeJSEnv(),
  )

lazy val esm = project
  .configure(fixture)
  .settings(scalaJSLinkerConfig ~= (_.withModuleKind(ModuleKind.ESModule)))

lazy val cjs = project
  .configure(fixture)
  .settings(
    scalaJSLinkerConfig ~= (_.withModuleKind(ModuleKind.CommonJSModule)),
    Test / jsEnv := new NodeJSEnv(NodeJSEnv.Config().withArgs(List("--max-old-space-size=300")).withEnv(Map("FIXTURE_GREETING" -> "hello"))),
  )

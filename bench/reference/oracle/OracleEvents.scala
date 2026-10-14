import sbt._
import sbt.Keys._

// Every event of sbt's test run as `teq test --events` writes teq's (src/task/events.rs, `Event::line`), a line each
// in target/oracle-events.txt under the build's root: the project, the event's name (its suite), its selector, its
// status (`sbt.testing.Status`) and the first line of what it threw, tab-separated. The oracle's inventory is read
// from it (bench/reference/reference.scala, `Tests.fromEvents`), each status as the framework reported it and a case
// as many times. Scala 2.12, the meta-build of sbt 1.
object OracleEvents extends AutoPlugin {
  override def trigger = allRequirements

  private def selector(s: sbt.testing.Selector): String = s match {
    case t: sbt.testing.TestSelector => "test " + t.testName
    case t: sbt.testing.NestedTestSelector => "nested " + t.suiteId + " " + t.testName
    case t: sbt.testing.NestedSuiteSelector => "nestedSuite " + t.suiteId
    case t: sbt.testing.TestWildcardSelector => "wildcard " + t.testWildcard
    case _ => "suite"
  }

  private def clean(s: String): String = s.replace('\t', ' ').replace('\n', ' ').replace('\r', ' ')

  private def recorder(project: String, file: File): TestReportListener = new TestReportListener {
    def doInit(): Unit = ()
    def startGroup(name: String): Unit = ()
    def testEvent(event: TestEvent): Unit = event.detail.foreach { e =>
      val thrown = if (e.throwable.isDefined) e.throwable.get.toString.linesIterator.toList.headOption.getOrElse("") else ""
      val line = Seq(project, e.fullyQualifiedName, selector(e.selector), e.status.toString, thrown).map(clean).mkString("\t") + "\n"
      OracleEvents.synchronized(IO.append(file, line))
    }
    def endGroup(name: String, result: TestResult): Unit = ()
    def endGroup(name: String, t: Throwable): Unit = ()
    def doComplete(result: TestResult): Unit = ()
  }

  override def projectSettings = Seq(
    Test / testListeners += recorder(thisProject.value.id, (ThisBuild / baseDirectory).value / "target" / "oracle-events.txt"),
  )
}

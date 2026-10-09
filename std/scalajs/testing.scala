// sbt's test interface as Scala.js has it (`scalajs-test-interface`, a Scala 2.13 artifact whose
// bodies are Scala.js IR alone, so no TASTy of it can be read): what a test framework's Scala.js
// jar implements (munit, zio-test-sbt, utest) and the test bridge (test_bridge.scala) drives.
// Adapted from Scala.js (https://www.scala-js.org/, Copyright EPFL, Apache License 2.0), release
// 1.22.0, the shapes kept as they are there. JavaScript only: the JVM has sbt's Java interface.
package sbt.testing

import scala.scalajs.reflect.annotation.EnableReflectiveInstantiation

trait Event:
  def fullyQualifiedName(): String
  def fingerprint(): Fingerprint
  def selector(): Selector
  def status(): Status
  def throwable(): OptionalThrowable
  def duration(): Long

trait EventHandler:
  def handle(event: Event): Unit

trait Fingerprint

trait AnnotatedFingerprint extends Fingerprint:
  def isModule(): Boolean
  def annotationName(): String

trait SubclassFingerprint extends Fingerprint:
  def isModule(): Boolean
  def superclassName(): String
  def requireNoArgConstructor(): Boolean

@EnableReflectiveInstantiation
trait Framework:
  def name(): String
  def fingerprints(): Array[Fingerprint]
  def runner(args: Array[String], remoteArgs: Array[String], testClassLoader: ClassLoader): Runner
  def slaveRunner(args: Array[String], remoteArgs: Array[String], testClassLoader: ClassLoader, send: String => Unit): Runner

trait Logger:
  def ansiCodesSupported(): Boolean
  def error(msg: String): Unit
  def warn(msg: String): Unit
  def info(msg: String): Unit
  def debug(msg: String): Unit
  def trace(t: Throwable): Unit

final class OptionalThrowable(private val exception: Throwable) extends Serializable:
  def this() = this(null)

  def isDefined(): Boolean = exception != null

  def isEmpty(): Boolean = exception == null

  def get(): Throwable =
    if exception == null then throw new IllegalStateException("This OptionalThrowable is not defined")
    else exception

  override def equals(that: Any): Boolean = that match
    case that: OptionalThrowable => this.exception eq that.exception
    case _ => false

  override def hashCode(): Int = if exception == null then 0 else exception.hashCode()

  override def toString(): String =
    if exception == null then "OptionalThrowable()" else s"OptionalThrowable($exception)"

trait Runner:
  def tasks(taskDefs: Array[TaskDef]): Array[Task]
  def done(): String
  def remoteArgs(): Array[String]
  def args: Array[String]
  def receiveMessage(msg: String): Option[String]
  def serializeTask(task: Task, serializer: TaskDef => String): String
  def deserializeTask(task: String, deserializer: String => TaskDef): Task

abstract sealed class Selector

final class SuiteSelector extends Selector with Serializable:
  override def equals(o: Any): Boolean = o.isInstanceOf[SuiteSelector]
  override def hashCode(): Int = 29
  override def toString(): String = "SuiteSelector"

final class TestSelector(_testName: String) extends Selector with Serializable:
  if _testName == null then throw new NullPointerException("testName was null")

  def testName(): String = _testName

  override def equals(that: Any): Boolean = that match
    case that: TestSelector => this.testName() == that.testName()
    case _ => false

  override def hashCode(): Int = testName().hashCode()
  override def toString(): String = s"TestSelector(${testName()})"

final class NestedSuiteSelector(_suiteId: String) extends Selector with Serializable:
  if _suiteId == null then throw new NullPointerException("suiteId was null")

  def suiteId(): String = _suiteId

  override def equals(that: Any): Boolean = that match
    case that: NestedSuiteSelector => this.suiteId() == that.suiteId()
    case _ => false

  override def hashCode(): Int = suiteId().hashCode()
  override def toString(): String = s"NestedSuiteSelector(${suiteId()})"

final class NestedTestSelector(_suiteId: String, _testName: String) extends Selector with Serializable:
  if _suiteId == null then throw new NullPointerException("suiteId was null")
  if _testName == null then throw new NullPointerException("testName was null")

  def suiteId(): String = _suiteId

  def testName(): String = _testName

  override def equals(that: Any): Boolean = that match
    case that: NestedTestSelector => this.suiteId() == that.suiteId() && this.testName() == that.testName()
    case _ => false

  override def hashCode(): Int =
    var retVal = 17
    retVal = 31 * retVal + suiteId().hashCode()
    retVal = 31 * retVal + testName().hashCode()
    retVal

  override def toString(): String = s"NestedTestSelector(${suiteId()}, ${testName()})"

final class TestWildcardSelector(_testWildcard: String) extends Selector with Serializable:
  if _testWildcard == null then throw new NullPointerException("testWildcard was null")

  def testWildcard(): String = _testWildcard

  override def equals(that: Any): Boolean = that match
    case that: TestWildcardSelector => this.testWildcard() == that.testWildcard()
    case _ => false

  override def hashCode(): Int = testWildcard().hashCode()

  override def toString(): String = s"TestWildcardSelector(${testWildcard()})"

// Java-style, as Scala.js writes it: an `Enum` whose values pass their name and ordinal.
class Status private (name: String, ordinal: Int) extends Enum[Status](name, ordinal)

object Status:
  final val Success = new Status("Success", 0)
  final val Error = new Status("Error", 1)
  final val Failure = new Status("Failure", 2)
  final val Skipped = new Status("Skipped", 3)
  final val Ignored = new Status("Ignored", 4)
  final val Canceled = new Status("Canceled", 5)
  final val Pending = new Status("Pending", 6)

  private val _values: Array[Status] = Array(Success, Error, Failure, Skipped, Ignored, Canceled, Pending)

  def values(): Array[Status] = _values.clone()

  def valueOf(name: String): Status =
    _values.find(_.name == name).getOrElse(throw new IllegalArgumentException("No enum const Status." + name))

trait Task:
  def tags(): Array[String]
  def execute(eventHandler: EventHandler, loggers: Array[Logger]): Array[Task]
  def execute(eventHandler: EventHandler, loggers: Array[Logger], continuation: Array[Task] => Unit): Unit
  def taskDef(): TaskDef

final class TaskDef(_fullyQualifiedName: String, _fingerprint: Fingerprint, _explicitlySpecified: Boolean, _selectors: Array[Selector]) extends Serializable:
  if _fullyQualifiedName == null then throw new NullPointerException("fullyQualifiedName was null")
  if _fingerprint == null then throw new NullPointerException("fingerprint was null")
  if _selectors == null then throw new NullPointerException("selectors was null")

  def fullyQualifiedName(): String = _fullyQualifiedName

  def fingerprint(): Fingerprint = _fingerprint

  def explicitlySpecified(): Boolean = _explicitlySpecified

  def selectors(): Array[Selector] = _selectors

  override def equals(that: Any): Boolean = that match
    case that: TaskDef =>
      this.fullyQualifiedName() == that.fullyQualifiedName() &&
      this.fingerprint() == that.fingerprint() &&
      this.explicitlySpecified() == that.explicitlySpecified() &&
      java.util.Arrays.equals(this.selectors().asInstanceOf[Array[AnyRef]], that.selectors().asInstanceOf[Array[AnyRef]])
    case _ => false

  override def hashCode(): Int =
    var retVal = 17
    retVal = 31 * retVal + _fullyQualifiedName.hashCode()
    retVal = 31 * retVal + _fingerprint.hashCode()
    retVal = 31 * retVal + (if _explicitlySpecified then 1 else 0)
    retVal = 31 * retVal + java.util.Arrays.hashCode(_selectors.asInstanceOf[Array[AnyRef]])
    retVal

  override def toString(): String =
    "TaskDef(" + _fullyQualifiedName + ", " + _fingerprint + ", " + _explicitlySpecified + ", " + _selectors.mkString("[", ", ", "]") + ")"

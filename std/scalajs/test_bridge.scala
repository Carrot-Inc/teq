// Scala.js's test bridge (`scalajs-test-bridge`, a Scala 2.13 artifact whose bodies are Scala.js
// IR alone): what runs under node when sbt's `test` of a Scala.js project runs, and speaks to
// sbt-scalajs's test adapter over the JS environment's com channel (`scalajsCom`). The adapter
// asks it which of sbt's framework names the program holds, starts runners, hands it tasks to
// run and takes their events and log lines back. Adapted from Scala.js
// (https://www.scala-js.org/, Copyright EPFL, Apache License 2.0), release 1.22.0: the messages
// are the same bytes; the endpoints take their message types as type parameters where Scala.js
// declares them as type members, and the HTML runner (`testHtml`) is left out.
package org.scalajs.testing.common:

  import java.io.{ByteArrayInputStream, ByteArrayOutputStream, DataInputStream, DataOutputStream, IOException}
  import sbt.testing.*

  private[testing] trait Serializer[T]:
    def serialize(x: T, out: Serializer.SerializeState): Unit
    def deserialize(in: Serializer.DeserializeState): T

  private[testing] object Serializer:
    final class SerializeState private[Serializer] (val out: DataOutputStream):
      def write[T](t: T)(implicit s: Serializer[T]): Unit = s.serialize(t, this)

    final class DeserializeState private[Serializer] (val in: DataInputStream):
      def read[T]()(implicit s: Serializer[T]): T = s.deserialize(this)

    def serialize[T](t: T, out: DataOutputStream)(implicit s: Serializer[T]): Unit =
      s.serialize(t, new SerializeState(out))

    def deserialize[T](in: DataInputStream)(implicit s: Serializer[T]): T =
      s.deserialize(new DeserializeState(in))

    def serialize[T: Serializer](t: T): String =
      withOutputStream(out => Serializer.serialize(t, out))

    def deserialize[T: Serializer](s: String): T =
      withInputStream(s)(in => Serializer.deserialize[T](in))

    // A message is a string of one char per byte.
    def withInputStream[T](s: String)(body: DataInputStream => T): T =
      val bytes = s.toArray.map(_.toByte)
      val in = new DataInputStream(new ByteArrayInputStream(bytes))
      try body(in)
      finally in.close()

    def withOutputStream(body: DataOutputStream => Unit): String =
      val byteOut = new ByteArrayOutputStream()
      val dataOut = new DataOutputStream(byteOut)
      try body(dataOut)
      finally dataOut.close()
      new String(byteOut.toByteArray().map(b => (b & 0xff).toChar))

    implicit object BooleanSerializer extends Serializer[Boolean]:
      def serialize(x: Boolean, out: SerializeState): Unit = out.out.writeBoolean(x)
      def deserialize(in: DeserializeState): Boolean = in.in.readBoolean()

    implicit object ByteSerializer extends Serializer[Byte]:
      def serialize(x: Byte, out: SerializeState): Unit = out.out.writeByte(x)
      def deserialize(in: DeserializeState): Byte = in.in.readByte()

    implicit object IntSerializer extends Serializer[Int]:
      def serialize(x: Int, out: SerializeState): Unit = out.out.writeInt(x)
      def deserialize(in: DeserializeState): Int = in.in.readInt()

    implicit object LongSerializer extends Serializer[Long]:
      def serialize(x: Long, out: SerializeState): Unit = out.out.writeLong(x)
      def deserialize(in: DeserializeState): Long = in.in.readLong()

    // `writeUTF` with the length in four bytes, for strings longer than `Short.MaxValue`.
    implicit object StringSerializer extends Serializer[String]:
      def serialize(x: String, out: SerializeState): Unit =
        out.out.writeInt(x.length)
        var i = 0
        while i < x.length do
          val c = x.charAt(i)
          if c <= 0x7f && c >= 0x01 then out.out.write(c.toInt)
          else if c < 0x0800 then
            out.out.write((c >> 6) | 0xc0)
            out.out.write((c & 0x3f) | 0x80)
          else
            out.out.write((c >> 12) | 0xe0)
            out.out.write(((c >> 6) & 0x3f) | 0x80)
            out.out.write((c & 0x3f) | 0x80)
          i += 1

      def deserialize(in: DeserializeState): String =
        val chars = Array.fill(in.in.readInt()) {
          val a = in.in.readByte()
          if (a & 0x80) == 0x00 then a.toChar
          else if (a & 0xe0) == 0xc0 then
            val b = in.in.readByte()
            require((b & 0xc0) == 0x80)
            (((a & 0x1f) << 6) | (b & 0x3f)).toChar
          else if (a & 0xf0) == 0xe0 then
            val b = in.in.readByte()
            val c = in.in.readByte()
            require((b & 0xc0) == 0x80)
            require((c & 0xc0) == 0x80)
            (((a & 0x0f) << 12) | ((b & 0x3f) << 6) | (c & 0x3f)).toChar
          else throw new IllegalArgumentException(s"bad byte: $a")
        }
        new String(chars)

    implicit object UnitSerializer extends Serializer[Unit]:
      def serialize(x: Unit, out: SerializeState): Unit = ()
      def deserialize(in: DeserializeState): Unit = ()

    implicit def listSerializer[T: Serializer]: Serializer[List[T]] = new Serializer[List[T]]:
      def serialize(x: List[T], out: SerializeState): Unit =
        out.write(x.size)
        x.foreach(out.write(_))
      def deserialize(in: DeserializeState): List[T] = List.fill(in.read[Int]())(in.read[T]())

    implicit def optionSerializer[T: Serializer]: Serializer[Option[T]] = new Serializer[Option[T]]:
      def serialize(x: Option[T], out: SerializeState): Unit =
        out.write(x.isDefined)
        x.foreach(out.write(_))
      def deserialize(in: DeserializeState): Option[T] =
        if in.read[Boolean]() then Some(in.read[T]()) else None

    implicit object StackTraceElementSerializer extends Serializer[StackTraceElement]:
      def serialize(x: StackTraceElement, out: SerializeState): Unit =
        out.write(x.getClassName)
        out.write(x.getMethodName)
        out.write(Option(x.getFileName))
        out.write(x.getLineNumber)
      def deserialize(in: DeserializeState): StackTraceElement =
        new StackTraceElement(in.read[String](), in.read[String](), in.read[Option[String]]().getOrElse(null), in.read[Int]())

    implicit object ThrowableSerializer extends Serializer[Throwable]:
      def serialize(x: Throwable, out: SerializeState): Unit =
        out.write(Option(x.getMessage()))
        out.write(x.toString())
        out.write(x.getStackTrace.toList)
        out.write(Option(x.getCause()))
      def deserialize(in: DeserializeState): Throwable =
        val msg = in.read[Option[String]]().getOrElse(null)
        val toStr = in.read[String]()
        val trace = in.read[List[StackTraceElement]]()
        val cause = in.read[Option[Throwable]]()
        val res = new Throwable(msg, cause.getOrElse(null)):
          override def toString(): String = toStr
        res.setStackTrace(trace.toArray)
        res

    implicit object FingerprintSerializer extends Serializer[Fingerprint]:
      private val Annotated: Byte = 1
      private val Subclass: Byte = 2

      def serialize(fp: Fingerprint, out: SerializeState): Unit = fp match
        case fp: AnnotatedFingerprint =>
          out.write(Annotated)
          out.write(fp.isModule())
          out.write(fp.annotationName())
        case fp: SubclassFingerprint =>
          out.write(Subclass)
          out.write(fp.isModule())
          out.write(fp.superclassName())
          out.write(fp.requireNoArgConstructor())
        case _ => throw new IllegalArgumentException(s"Unknown Fingerprint type: ${fp.getClass()}")

      // The fields read in their order, as Scala.js's `val`s of the anonymous class read them.
      def deserialize(in: DeserializeState): Fingerprint = in.read[Byte]() match
        case Annotated =>
          val module = in.read[Boolean]()
          val annotation = in.read[String]()
          new AnnotatedFingerprint:
            def isModule(): Boolean = module
            def annotationName(): String = annotation
        case Subclass =>
          val module = in.read[Boolean]()
          val superclass = in.read[String]()
          val noArgs = in.read[Boolean]()
          new SubclassFingerprint:
            def isModule(): Boolean = module
            def superclassName(): String = superclass
            def requireNoArgConstructor(): Boolean = noArgs
        case t => throw new IOException(s"Unknown Fingerprint type: $t")

    implicit object SelectorSerializer extends Serializer[Selector]:
      private val Suite: Byte = 1
      private val Test: Byte = 2
      private val NestedSuite: Byte = 3
      private val NestedTest: Byte = 4
      private val TestWildcard: Byte = 5

      def serialize(sel: Selector, out: SerializeState): Unit = sel match
        case sel: SuiteSelector => out.write(Suite)
        case sel: TestSelector =>
          out.write(Test)
          out.write(sel.testName())
        case sel: NestedSuiteSelector =>
          out.write(NestedSuite)
          out.write(sel.suiteId())
        case sel: NestedTestSelector =>
          out.write(NestedTest)
          out.write(sel.suiteId())
          out.write(sel.testName())
        case sel: TestWildcardSelector =>
          out.write(TestWildcard)
          out.write(sel.testWildcard())
        case _ => throw new IllegalArgumentException(s"Unknown Selector type: ${sel.getClass()}")

      def deserialize(in: DeserializeState): Selector = in.read[Byte]() match
        case Suite => new SuiteSelector()
        case Test => new TestSelector(in.read[String]())
        case NestedSuite => new NestedSuiteSelector(in.read[String]())
        case NestedTest => new NestedTestSelector(in.read[String](), in.read[String]())
        case TestWildcard => new TestWildcardSelector(in.read[String]())
        case t => throw new IOException(s"Unknown Selector type: $t")

    implicit object TaskDefSerializer extends Serializer[TaskDef]:
      def serialize(x: TaskDef, out: SerializeState): Unit =
        out.write(x.fullyQualifiedName())
        out.write(x.fingerprint())
        out.write(x.explicitlySpecified())
        out.write(x.selectors().toList)
      def deserialize(in: DeserializeState): TaskDef =
        new TaskDef(in.read[String](), in.read[Fingerprint](), in.read[Boolean](), in.read[List[Selector]]().toArray)

    implicit object StatusSerializer extends Serializer[Status]:
      def serialize(x: Status, out: SerializeState): Unit = out.write(x.ordinal())
      def deserialize(in: DeserializeState): Status =
        val values = Status.values()
        val ord = in.read[Int]()
        if ord < 0 || ord >= values.length then throw new IOException(s"Got bad status ordinal: $ord")
        values(ord)

    implicit object OptionalThrowableSerializer extends Serializer[OptionalThrowable]:
      def serialize(x: OptionalThrowable, out: SerializeState): Unit =
        out.write(x.isDefined())
        if x.isDefined() then out.write(x.get())
      def deserialize(in: DeserializeState): OptionalThrowable =
        if in.read[Boolean]() then new OptionalThrowable(in.read[Throwable]()) else new OptionalThrowable()

    implicit object EventSerializer extends Serializer[Event]:
      def serialize(x: Event, out: SerializeState): Unit =
        out.write(x.fullyQualifiedName())
        out.write(x.fingerprint())
        out.write(x.selector())
        out.write(x.status())
        out.write(x.throwable())
        out.write(x.duration())
      def deserialize(in: DeserializeState): Event =
        val name = in.read[String]()
        val fp = in.read[Fingerprint]()
        val sel = in.read[Selector]()
        val st = in.read[Status]()
        val thrown = in.read[OptionalThrowable]()
        val took = in.read[Long]()
        new Event:
          def fullyQualifiedName(): String = name
          def fingerprint(): Fingerprint = fp
          def selector(): Selector = sel
          def status(): Status = st
          def throwable(): OptionalThrowable = thrown
          def duration(): Long = took

  private[testing] sealed trait Endpoint:
    val opCode: RPCCore.OpCode

  private[testing] final class MsgEndpoint[M](val opCode: RPCCore.OpCode)(implicit val msgSerializer: Serializer[M]) extends Endpoint:
    require(!RPCCore.isReservedOpCode(opCode), s"Reserved op code: $opCode")

  private[testing] final class RPCEndpoint[Rq, Rp](val opCode: RPCCore.OpCode)(implicit val reqSerializer: Serializer[Rq], val respSerializer: Serializer[Rp]) extends Endpoint:
    require(!RPCCore.isReservedOpCode(opCode), s"Reserved op code: $opCode")

  private[testing] final class ExecuteRequest(val taskInfo: TaskInfo, val loggerColorSupport: List[Boolean])

  private[testing] object ExecuteRequest:
    implicit object ExecuteRequestSerializer extends Serializer[ExecuteRequest]:
      def serialize(x: ExecuteRequest, out: Serializer.SerializeState): Unit =
        out.write(x.taskInfo)
        out.write(x.loggerColorSupport)
      def deserialize(in: Serializer.DeserializeState): ExecuteRequest =
        new ExecuteRequest(in.read[TaskInfo](), in.read[List[Boolean]]())

  private[testing] final class FrameworkInfo(val implName: String, val displayName: String, val fingerprints: List[Fingerprint])

  private[testing] object FrameworkInfo:
    implicit object FrameworkInfoSerializer extends Serializer[FrameworkInfo]:
      def serialize(x: FrameworkInfo, out: Serializer.SerializeState): Unit =
        out.write(x.implName)
        out.write(x.displayName)
        out.write(x.fingerprints)
      def deserialize(in: Serializer.DeserializeState): FrameworkInfo =
        new FrameworkInfo(in.read[String](), in.read[String](), in.read[List[Fingerprint]]())

  private[testing] final class FrameworkMessage(val workerId: Long, val msg: String)

  private[testing] object FrameworkMessage:
    implicit object FrameworkMessageSerializer extends Serializer[FrameworkMessage]:
      def serialize(x: FrameworkMessage, out: Serializer.SerializeState): Unit =
        out.write(x.workerId)
        out.write(x.msg)
      def deserialize(in: Serializer.DeserializeState): FrameworkMessage =
        new FrameworkMessage(in.read[Long](), in.read[String]())

  private[testing] final class IsolatedTestSet(val testFrameworkNames: List[List[String]], val definedTests: List[TaskDef])

  private[testing] object IsolatedTestSet:
    implicit object IsolatedTestSetSerializer extends Serializer[IsolatedTestSet]:
      def serialize(x: IsolatedTestSet, out: Serializer.SerializeState): Unit =
        out.write(x.testFrameworkNames)
        out.write(x.definedTests)
      def deserialize(in: Serializer.DeserializeState): IsolatedTestSet =
        new IsolatedTestSet(in.read[List[List[String]]](), in.read[List[TaskDef]]())

  private[testing] final class LogElement[T](val index: Int, val x: T)

  private[testing] object LogElement:
    implicit def logElementSerializer[T: Serializer]: Serializer[LogElement[T]] = new Serializer[LogElement[T]]:
      def serialize(x: LogElement[T], out: Serializer.SerializeState): Unit =
        out.write(x.index)
        out.write(x.x)
      def deserialize(in: Serializer.DeserializeState): LogElement[T] =
        new LogElement(in.read[Int](), in.read[T]())

  private[testing] final class RunMux[+T](val runId: RunMux.RunID, val value: T)

  private[testing] object RunMux:
    type RunID = Int

    implicit def runMuxSerializer[T: Serializer]: Serializer[RunMux[T]] = new Serializer[RunMux[T]]:
      def serialize(x: RunMux[T], out: Serializer.SerializeState): Unit =
        out.write(x.runId)
        out.write(x.value)
      def deserialize(in: Serializer.DeserializeState): RunMux[T] =
        new RunMux(in.read[Int](), in.read[T]())

  private[testing] final class RunnerArgs(val runID: RunMux.RunID, val frameworkImpl: String, val args: List[String], val remoteArgs: List[String])

  private[testing] object RunnerArgs:
    implicit object RunnerArgsSerializer extends Serializer[RunnerArgs]:
      def serialize(x: RunnerArgs, out: Serializer.SerializeState): Unit =
        out.write(x.runID)
        out.write(x.frameworkImpl)
        out.write(x.args)
        out.write(x.remoteArgs)
      def deserialize(in: Serializer.DeserializeState): RunnerArgs =
        new RunnerArgs(in.read[Int](), in.read[String](), in.read[List[String]](), in.read[List[String]]())

  private[testing] final class TaskInfo(val serializedTask: String, val taskDef: TaskDef, val tags: List[String])

  private[testing] object TaskInfo:
    implicit object TaskInfoSerializer extends Serializer[TaskInfo]:
      def serialize(x: TaskInfo, out: Serializer.SerializeState): Unit =
        out.write(x.serializedTask)
        out.write(x.taskDef)
        out.write(x.tags)
      def deserialize(in: Serializer.DeserializeState): TaskInfo =
        new TaskInfo(in.read[String](), in.read[TaskDef](), in.read[List[String]]())

  private[testing] sealed abstract class TestBridgeMode

  private[testing] object TestBridgeMode:
    case object FullBridge extends TestBridgeMode
    final case class HTMLRunner(tests: IsolatedTestSet) extends TestBridgeMode

    implicit object TestBridgeModeSerializer extends Serializer[TestBridgeMode]:
      def serialize(x: TestBridgeMode, out: Serializer.SerializeState): Unit = x match
        case FullBridge => out.write(0)
        case HTMLRunner(tests) =>
          out.write(1)
          out.write(tests)
      def deserialize(in: Serializer.DeserializeState): TestBridgeMode = in.read[Int]() match
        case 0 => FullBridge
        case 1 => HTMLRunner(in.read[IsolatedTestSet]())
        case n => throw new IOException(s"Unknown bridge mode: $n")

  // The endpoints of the bridge (the JavaScript side) and of the adapter (the JVM side), by
  // their op codes.
  private[testing] object JSEndpoints:
    val detectFrameworks: RPCEndpoint[List[List[String]], List[Option[FrameworkInfo]]] = new RPCEndpoint(2)
    val createControllerRunner: RPCEndpoint[RunnerArgs, Unit] = new RPCEndpoint(3)
    val createWorkerRunner: RPCEndpoint[RunnerArgs, Unit] = new RPCEndpoint(4)
    val msgWorker: MsgEndpoint[RunMux[String]] = new MsgEndpoint(5)
    val msgController: MsgEndpoint[RunMux[FrameworkMessage]] = new MsgEndpoint(6)
    val tasks: RPCEndpoint[RunMux[List[TaskDef]], List[TaskInfo]] = new RPCEndpoint(7)
    val execute: RPCEndpoint[RunMux[ExecuteRequest], List[TaskInfo]] = new RPCEndpoint(8)
    val done: RPCEndpoint[RunMux[Unit], String] = new RPCEndpoint(9)

  private[testing] object JVMEndpoints:
    val msgWorker: MsgEndpoint[RunMux[String]] = new MsgEndpoint(2)
    val msgController: MsgEndpoint[RunMux[FrameworkMessage]] = new MsgEndpoint(3)
    val event: MsgEndpoint[RunMux[Event]] = new MsgEndpoint(4)
    val logError: MsgEndpoint[RunMux[LogElement[String]]] = new MsgEndpoint(5)
    val logWarn: MsgEndpoint[RunMux[LogElement[String]]] = new MsgEndpoint(6)
    val logInfo: MsgEndpoint[RunMux[LogElement[String]]] = new MsgEndpoint(7)
    val logDebug: MsgEndpoint[RunMux[LogElement[String]]] = new MsgEndpoint(8)
    val logTrace: MsgEndpoint[RunMux[LogElement[Throwable]]] = new MsgEndpoint(9)

  /** The dispatcher of calls and messages over a message passing interface: a call gets an
    * identity its reply names; a message or a call that arrives goes to the endpoint attached
    * for its op code, synchronously, so that `close` is free of races. */
  private[testing] abstract class RPCCore()(implicit ec: scala.concurrent.ExecutionContext):
    import scala.concurrent.{Future, Promise}
    import scala.util.{Failure, Success, Try}
    import RPCCore.*

    private val pending = new java.util.concurrent.ConcurrentHashMap[Long, PendingCall[?]]
    @volatile private var closeReason: Throwable = null
    private val nextID = new java.util.concurrent.atomic.AtomicLong(0L)
    private val endpoints = new java.util.concurrent.ConcurrentHashMap[OpCode, BoundEndpoint]

    final protected def handleMessage(msg: String): Unit =
      Serializer.withInputStream(msg) { in =>
        val opCode = in.readByte()

        def getPending(): Option[PendingCall[?]] =
          val callID = in.readLong()
          Option(pending.remove(callID))

        opCode match
          case RPCCore.ReplyOK => getPending().foreach(_.complete(in))
          case RPCCore.ReplyErr =>
            getPending().foreach { p =>
              val throwable = Try(Serializer.deserialize[Throwable](in)) match
                case Success(t) => new RPCException(t)
                case Failure(t) => t
              p.fail(throwable)
            }
          case _ =>
            endpoints.get(opCode) match
              case null =>
                val detail =
                  if opCode == JSEndpoints.msgWorker.opCode then
                    "; The test adapter could not send a message to a worker, which probably happens because the worker terminated early, " +
                      "without waiting for the reply to a call to send(). This is probably a bug in the testing framework you are using. See also #3201."
                  else ""
                throw new IllegalStateException(s"Unknown opcode: $opCode$detail")
              case bep: BoundMsgEndpoint[?] => bep.handle(in)
              case bep: BoundRPCEndpoint[?, ?] =>
                val callID = in.readLong()
                bep.handle(in).onComplete(repl => send(bep.reply(callID, repl)))
      }

    protected def send(msg: String): Unit

    final def send[M](ep: MsgEndpoint[M])(msg: M): Unit =
      send(makeMsgMsg(ep.opCode, msg)(using ep.msgSerializer))

    final def call[Rq, Rp](ep: RPCEndpoint[Rq, Rp])(req: Rq): Future[Rp] =
      val id = nextID.incrementAndGet()
      val msg = makeRPCMsg(ep.opCode, id, req)(using ep.reqSerializer)
      val promise = Promise[Rp]()
      val oldCall = pending.put(id, new PendingCall(promise, ep.respSerializer))
      if oldCall != null then
        val error = new AssertionError("Ran out of call ids!")
        close(error)
        throw error
      if closeReason != null then helpClose()
      else send(msg)
      promise.future

    final def attach[M](ep: MsgEndpoint[M])(ex: M => Unit): Unit =
      attachBound(new BoundMsgEndpoint(ep, ex))

    final def attach[Rq, Rp](ep: RPCEndpoint[Rq, Rp])(ex: Rq => Rp): Unit =
      attachAsync(ep)(x => Future.fromTry(Try(ex(x))))

    final def attachAsync[Rq, Rp](ep: RPCEndpoint[Rq, Rp])(ex: Rq => Future[Rp]): Unit =
      attachBound(new BoundRPCEndpoint(ep, ex))

    private final def attachBound(bep: BoundEndpoint): Unit =
      val opCode = bep.endpoint.opCode
      val old = endpoints.put(opCode, bep)
      require(old == null, s"Duplicate endpoint for opcode $opCode.")

    final def detach(ep: Endpoint): Unit =
      val old = endpoints.remove(ep.opCode)
      require(old != null, "Endpoint was not attached.")

    def close(reason: Throwable): Unit =
      closeReason = reason
      helpClose()

    private def helpClose(): Unit =
      val exception = new ClosedException(closeReason)
      val ids = pending.keySet().iterator()
      while ids.hasNext() do
        val callID = ids.next()
        for failing <- Option(pending.remove(callID)) do failing.fail(exception)

  private[testing] object RPCCore:
    import scala.concurrent.{Future, Promise}
    import scala.util.{Failure, Success, Try}

    type OpCode = Byte

    final case class RPCException(c: Throwable) extends Exception(c)
    final case class ClosedException(c: Throwable) extends Exception(c)

    private[common] val ReplyOK: Byte = 0.toByte
    private[common] val ReplyErr: Byte = 1.toByte

    def isReservedOpCode(opc: OpCode): Boolean = opc == ReplyOK || opc == ReplyErr

    private[common] def makeReply[T](id: Long, result: Try[T])(implicit s: Serializer[T]): String =
      result.map(makeRPCMsg(ReplyOK, id, _)) match
        case Success(m) => m
        case Failure(t) => makeRPCMsg(ReplyErr, id, t)

    private[common] def makeRPCMsg[T](opCode: OpCode, id: Long, payload: T)(implicit s: Serializer[T]): String =
      Serializer.withOutputStream { out =>
        out.writeByte(opCode)
        out.writeLong(id)
        Serializer.serialize(payload, out)
      }

    private[common] def makeMsgMsg[T](opCode: OpCode, payload: T)(implicit s: Serializer[T]): String =
      Serializer.withOutputStream { out =>
        out.writeByte(opCode)
        Serializer.serialize(payload, out)
      }

    private[common] sealed abstract class BoundEndpoint:
      val endpoint: Endpoint

    private[common] final class BoundMsgEndpoint[M](val endpoint: MsgEndpoint[M], exec: M => Unit) extends BoundEndpoint:
      def handle(in: java.io.DataInputStream): Unit = exec(Serializer.deserialize(in)(using endpoint.msgSerializer))

    private[common] final class BoundRPCEndpoint[Rq, Rp](val endpoint: RPCEndpoint[Rq, Rp], exec: Rq => Future[Rp]) extends BoundEndpoint:
      def handle(in: java.io.DataInputStream)(implicit ec: scala.concurrent.ExecutionContext): Future[Rp] =
        Future.fromTry(Try(Serializer.deserialize(in)(using endpoint.reqSerializer))).flatMap(exec)
      def reply(callID: Long, result: Try[Rp]): String = makeReply(callID, result)(using endpoint.respSerializer)

    private[common] final class PendingCall[R](promise: Promise[R], serializer: Serializer[R]):
      def complete(in: java.io.DataInputStream): Unit = promise.complete(Try(Serializer.deserialize(in)(using serializer)))
      def fail(t: Throwable): Unit = promise.failure(t)

  /** An `RPCCore` that multiplexes between runs: an endpoint attached once per run. */
  private[testing] final class RunMuxRPC(rpc: RPCCore):
    import scala.collection.mutable
    import scala.concurrent.Future
    import scala.util.Try
    import RunMux.RunID

    private val mux = mutable.Map.empty[RPCCore.OpCode, java.util.concurrent.ConcurrentHashMap[RunID, Any]]

    def call[Req, Resp](ep: RPCEndpoint[RunMux[Req], Resp], runId: RunID)(req: Req): Future[Resp] =
      rpc.call(ep)(new RunMux(runId, req))

    def send[Msg](ep: MsgEndpoint[RunMux[Msg]], runId: RunID)(msg: Msg): Unit =
      rpc.send(ep)(new RunMux(runId, msg))

    def attach[Msg](ep: MsgEndpoint[RunMux[Msg]], runId: RunID)(ex: Msg => Unit): Unit =
      attachMux[Msg, Unit](ep.opCode, runId, ex)(f => rpc.attach(ep)(f))

    def attach[Req, Resp](ep: RPCEndpoint[RunMux[Req], Resp], runId: RunID)(ex: Req => Resp): Unit =
      attachAsync(ep, runId)(x => Future.fromTry(Try(ex(x))))

    def attachAsync[Req, Resp](ep: RPCEndpoint[RunMux[Req], Resp], runId: RunID)(ex: Req => Future[Resp]): Unit =
      attachMux[Req, Future[Resp]](ep.opCode, runId, ex)(f => rpc.attachAsync(ep)(f))

    private def attachMux[Req, Resp](opCode: RPCCore.OpCode, runId: RunID, ex: Req => Resp)(attach: (RunMux[Req] => Resp) => Unit): Unit = synchronized {
      def newDispatchMap() =
        val dispatch = new java.util.concurrent.ConcurrentHashMap[RunID, Any]
        attach { r =>
          Option(dispatch.get(r.runId)) match
            case None => throw new IllegalArgumentException(s"Unknown run ${r.runId}")
            case Some(f) => f.asInstanceOf[Req => Resp](r.value)
        }
        dispatch
      val dispatch = mux.getOrElseUpdate(opCode, newDispatchMap())
      val old = dispatch.put(runId, ex)
      require(old == null, s"Duplicate endpoint for opcode $opCode run $runId")
    }

    def detach(ep: Endpoint, runId: RunID): Unit = synchronized {
      val opCode = ep.opCode
      val dispatch = mux.getOrElse(opCode, throw new IllegalArgumentException(s"No endpoint attached for opCode $opCode"))
      val old = dispatch.remove(runId)
      require(old != null, s"No endpoint attached for opCode $opCode run $runId")
      if dispatch.isEmpty() then
        rpc.detach(ep)
        mux -= opCode
    }

package org.scalajs.testing.bridge:

  import scala.scalajs.js
  import scala.scalajs.js.annotation.JSGlobal
  import scala.scalajs.reflect.Reflect
  import scala.concurrent.{Future, Promise}
  import scala.util.Try
  import scala.util.control.NonFatal
  import org.scalajs.testing.common.*
  import sbt.testing.*

  // What the adapter's module initializer calls (`TestAdapterInitializer`): public, so that the
  // entry of a test link of teq's, which lives in a package of its own, can call it.
  object Bridge:
    def start(): Unit = mode match
      case TestBridgeMode.FullBridge => TestAdapterBridge.start()
      case TestBridgeMode.HTMLRunner(_) => throw new UnsupportedOperationException("the HTML test runner (testHtml) is not part of teq's test bridge")

    private def mode: TestBridgeMode =
      if js.typeOf(js.Dynamic.global.__ScalaJSTestBridgeMode) == "undefined" then TestBridgeMode.FullBridge
      else Serializer.deserialize[TestBridgeMode](js.Dynamic.global.__ScalaJSTestBridgeMode.asInstanceOf[String])

  // The com channel of the JS environment, over the queue execution context: the calls between
  // tests yield to a message or a handler in a finite number of steps.
  private[bridge] object JSRPC extends RPCCore()(using scala.scalajs.concurrent.JSExecutionContext.queue):
    Com.init(handleMessage _)

    override protected def send(msg: String): Unit = Com.send(msg)

    @js.native
    @JSGlobal("scalajsCom")
    private object Com extends js.Object:
      def init(onReceive: js.Function1[String, Unit]): Unit = js.native
      def send(msg: String): Unit = js.native

  private[bridge] object TestAdapterBridge:
    import scala.scalajs.concurrent.JSExecutionContext.Implicits.queue

    private val mux = new RunMuxRPC(JSRPC)

    def start(): Unit =
      JSRPC.attach(JSEndpoints.detectFrameworks)(detectFrameworksFun)
      JSRPC.attach(JSEndpoints.createControllerRunner)(createRunnerFun(isController = true))
      JSRPC.attach(JSEndpoints.createWorkerRunner)(createRunnerFun(isController = false))

    private def detectFrameworksFun = { (names: List[List[String]]) =>
      FrameworkLoader.detectFrameworkNames(names).map { maybeName =>
        maybeName.map { name =>
          val framework = FrameworkLoader.loadFramework(name)
          new FrameworkInfo(name, framework.name(), framework.fingerprints().toList)
        }
      }
    }

    private def createRunnerFun(isController: Boolean) = { (args: RunnerArgs) =>
      val framework = FrameworkLoader.loadFramework(args.frameworkImpl)
      val loader = new ScalaJSClassLoader()
      val runID = args.runID
      val runner =
        if isController then framework.runner(args.args.toArray, args.remoteArgs.toArray, loader)
        else framework.slaveRunner(args.args.toArray, args.remoteArgs.toArray, loader, mux.send(JVMEndpoints.msgWorker, runID))
      mux.attach(JSEndpoints.tasks, runID)(tasksFun(runner))
      mux.attachAsync(JSEndpoints.execute, runID)(executeFun(runID, runner))
      mux.attach(JSEndpoints.done, runID)(doneFun(runID, runner, isController))
      if isController then mux.attach(JSEndpoints.msgController, runID)(msgControllerFun(runID, runner))
      else mux.attach(JSEndpoints.msgWorker, runID)(runner.receiveMessage _)
    }

    private def detachRunnerCommands(runID: RunMux.RunID, isController: Boolean) =
      mux.detach(JSEndpoints.tasks, runID)
      mux.detach(JSEndpoints.execute, runID)
      mux.detach(JSEndpoints.done, runID)
      if isController then mux.detach(JSEndpoints.msgController, runID)
      else mux.detach(JSEndpoints.msgWorker, runID)

    private def tasksFun(runner: Runner) = { (taskDefs: List[TaskDef]) =>
      val tasks = runner.tasks(taskDefs.toArray)
      tasks.map(TaskInfoBuilder.detachTask(_, runner)).toList
    }

    private def executeFun(runID: RunMux.RunID, runner: Runner) = { (req: ExecuteRequest) =>
      val task = TaskInfoBuilder.attachTask(req.taskInfo, runner)
      val eventHandler = new RemoteEventHandler(runID)
      val loggers = for (withColor, i) <- req.loggerColorSupport.zipWithIndex yield new RemoteLogger(runID, i, withColor)
      val promise = Promise[List[TaskInfo]]()
      def cont(tasks: Array[Task]) =
        val result = Try(tasks.map(TaskInfoBuilder.detachTask(_, runner)).toList)
        promise.complete(result)
      try task.execute(eventHandler, loggers.toArray, cont)
      catch case NonFatal(t) => promise.tryFailure(t)
      promise.future
    }

    private def doneFun(runID: RunMux.RunID, runner: Runner, isController: Boolean) = { (_: Unit) =>
      try runner.done()
      finally detachRunnerCommands(runID, isController)
    }

    private def msgControllerFun(runID: RunMux.RunID, runner: Runner) = { (msg: FrameworkMessage) =>
      for reply <- runner.receiveMessage(msg.msg) do
        val fm = new FrameworkMessage(msg.workerId, reply)
        mux.send(JVMEndpoints.msgController, runID)(fm)
    }

    private class RemoteEventHandler(runID: RunMux.RunID) extends EventHandler:
      def handle(event: Event): Unit = mux.send(JVMEndpoints.event, runID)(event)

    private class RemoteLogger(runID: RunMux.RunID, index: Int, colors: Boolean) extends Logger:
      private def l[T](x: T) = new LogElement(index, x)
      def ansiCodesSupported(): Boolean = colors
      def error(msg: String): Unit = mux.send(JVMEndpoints.logError, runID)(l(msg))
      def warn(msg: String): Unit = mux.send(JVMEndpoints.logWarn, runID)(l(msg))
      def info(msg: String): Unit = mux.send(JVMEndpoints.logInfo, runID)(l(msg))
      def debug(msg: String): Unit = mux.send(JVMEndpoints.logDebug, runID)(l(msg))
      def trace(t: Throwable): Unit = mux.send(JVMEndpoints.logTrace, runID)(l(t))

  // Frameworks by their class names, as sbt names them: found and made through reflective
  // instantiation, which `Framework`'s annotation enables.
  private[bridge] object FrameworkLoader:
    def loadFramework(frameworkName: String): Framework =
      val clazz = Reflect.lookupInstantiatableClass(frameworkName).getOrElse(throw new InstantiationError(frameworkName))
      clazz.newInstance().asInstanceOf[Framework]

    def detectFrameworkNames(names: List[List[String]]): List[Option[String]] =
      def frameworkExists(name: String): Boolean =
        Reflect.lookupInstantiatableClass(name).exists(clazz => classOf[Framework].isAssignableFrom(clazz.runtimeClass))
      for frameworkNames <- names yield frameworkNames.find(frameworkExists(_))

    def tryLoadFramework(names: List[String]): Option[Framework] =
      def tryLoad(name: String): Option[Framework] =
        Reflect.lookupInstantiatableClass(name).collect {
          case clazz if classOf[Framework].isAssignableFrom(clazz.runtimeClass) => clazz.newInstance().asInstanceOf[Framework]
        }
      names.iterator.map(tryLoad).collectFirst { case Some(framework) => framework }

  // A class loader in name only, for the test interface's signatures: the classes are found
  // through `scala.scalajs.reflect.Reflect`.
  private[bridge] final class ScalaJSClassLoader extends ClassLoader(null)

  private[bridge] object TaskInfoBuilder:
    def detachTask(task: Task, runner: Runner): TaskInfo =
      def optSerializer(t: TaskDef) = if t == task.taskDef() then "" else Serializer.serialize(t)
      new TaskInfo(runner.serializeTask(task, optSerializer), task.taskDef(), task.tags().toList)

    def attachTask(info: TaskInfo, runner: Runner): Task =
      def optDeserializer(s: String) = if s == "" then info.taskDef else Serializer.deserialize[TaskDef](s)
      runner.deserializeTask(info.serializedTask, optDeserializer)

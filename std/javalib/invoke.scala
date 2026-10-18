// The JDK's method handles and `sun.misc.Signal` as far as a library reaches them for a program's signal handlers
// (zio's `Signal` finds `sun.misc.Signal`'s constructor and `handle` by method handles, its handler a `Proxy`):
// the interpreter's and JavaScript's, whose platform has them from here. The JVM has the JDK's own, which a
// program there calls, their members' JVM signatures the JDK's (`JS_ONLY_LAYER`).
package java.lang.invoke:
  // The JDK's method types and handles as far as a library finds a constructor or a static method of a class
  // reflectively and calls it (zio's `Signal` over `sun.misc.Signal`): the class's own constructor or its
  // object's method taking the parameters' classes, else the JDK's `NoSuchMethodException`; `invoke` and its
  // like on the arguments given.
  @jvmClass("java/lang/invoke/MethodType")
  final class MethodType private[invoke] (rtype: Class[?], ptypes: scala.Array[Class[?]]):
    @javaDefined
    def returnType(): Class[?] = rtype
    @javaDefined
    def parameterCount(): Int = ptypes.length
    def parameterType(num: Int): Class[?] = ptypes(num)
    @javaDefined
    def parameterArray(): scala.Array[Class[?]] = ptypes.clone()
    override def toString: String = ptypes.map(_.getSimpleName).mkString("(", ",", ")") + rtype.getSimpleName

  @jvmClass("java/lang/invoke/MethodType")
  object MethodType:
    def methodType(rtype: Class[?]): MethodType = new MethodType(rtype, new scala.Array[Class[?]](0))
    def methodType(rtype: Class[?], ptype0: Class[?]): MethodType = new MethodType(rtype, scala.Array[Class[?]](ptype0))
    def methodType(rtype: Class[?], ptype0: Class[?], ptypes: Class[?]*): MethodType = new MethodType(rtype, (ptype0 +: ptypes).toArray)
    def methodType(rtype: Class[?], ptypes: scala.Array[Class[?]]): MethodType = new MethodType(rtype, ptypes.clone())

  @jvmClass("java/lang/invoke/MethodHandle")
  abstract class MethodHandle private[invoke] ():
    @javaDefined
    def `type`(): MethodType
    def invoke(args: Any*): Any
    def invokeExact(args: Any*): Any = invoke(args*)
    def invokeWithArguments(args: Any*): Any = invoke(args*)

  private[invoke] final class ConstructorHandle(refc: Class[?], tpe: MethodType) extends MethodHandle:
    @javaDefined
    def `type`(): MethodType = tpe
    def invoke(args: Any*): Any = handleNew(refc, args.toArray[Any])

  private[invoke] final class StaticHandle(refc: Class[?], name: String, tpe: MethodType) extends MethodHandle:
    @javaDefined
    def `type`(): MethodType = tpe
    def invoke(args: Any*): Any = handleStatic(refc, name, args.toArray[Any])

  @jvmClass("java/lang/invoke/MethodHandles")
  object MethodHandles:
    @javaDefined
    def lookup(): Lookup = new Lookup()

    @jvmClass("java/lang/invoke/MethodHandles$Lookup")
    final class Lookup private[invoke] ():
      def findConstructor(refc: Class[?], tpe: MethodType): MethodHandle =
        if !handleFound(refc, null, tpe.parameterArray()) then
          throw new NoSuchMethodException("no such constructor: " + refc.getName + ".<init>" + tpe + "/newInvokeSpecial")
        new ConstructorHandle(refc, tpe)
      def findStatic(refc: Class[?], name: String, tpe: MethodType): MethodHandle =
        if !handleFound(refc, name, tpe.parameterArray()) then
          throw new NoSuchMethodException("no such method: " + refc.getName + "." + name + tpe + "/invokeStatic")
        new StaticHandle(refc, name, tpe)

  // Whether the class has the constructor (`name` null) or its object the method taking those classes.
  @js("$fail(\"UnsupportedOperationException\", \"reflection is not available on JavaScript\")")
  private[invoke] def handleFound(refc: Class[?], name: String, ptypes: scala.Array[Class[?]]): scala.Boolean
  @js("$fail(\"UnsupportedOperationException\", \"reflection is not available on JavaScript\")")
  private[invoke] def handleNew(refc: Class[?], args: scala.Array[Any]): AnyRef
  @js("$fail(\"UnsupportedOperationException\", \"reflection is not available on JavaScript\")")
  private[invoke] def handleStatic(refc: Class[?], name: String, args: scala.Array[Any]): Any

package sun.misc:
  // The JDK's `sun.misc.Signal` over `jdk.internal.misc.Signal`: a signal by its short name (`USR1`), refused
  // where the platform knows no such name or it starts with `SIG`; `handle` installs a handler, refused for a
  // signal the VM or the system keeps (SEGV, BUS, FPE, ILL, QUIT, KILL and STOP; on Windows all but INT, TERM,
  // BREAK and ABRT), answering the one it replaces: the program's, the shutdown's (INT, TERM and HUP's at the
  // start), `SIG_DFL` or `SIG_IGN`. A signal the program handles is dispatched as the JDK's `Signal.dispatch`
  // dispatches it, its handler run on a thread of its own, `SIG<name> handler`, a daemon as the dispatcher is
  // (src/interp/process.rs, `dispatch_signal`); a handled INT or TERM no longer begins the shutdown.
  @jvmClass("sun/misc/Signal")
  final class Signal(name: String):
    if name == null then throw new NullPointerException("name")
    if name.startsWith("SIG") then throw new IllegalArgumentException("Unknown signal: " + name)
    private val number = signalNumber(name)
    if number < 0 then throw new IllegalArgumentException("Unknown signal: " + name)
    @javaDefined
    @jvm("invokevirtual sun/misc/Signal.getName()Ljava/lang/String;")
    def getName(): String = name
    @javaDefined
    @jvm("invokevirtual sun/misc/Signal.getNumber()I")
    def getNumber(): Int = number
    override def equals(other: Any): scala.Boolean = other match
      case s: Signal => s.getName() == name && s.getNumber() == number
      case _ => false
    override def hashCode: Int = number
    override def toString: String = "SIG" + name

  @jvmClass("sun/misc/Signal")
  object Signal:
    private val handlers = new java.util.HashMap[Int, SignalHandler]()
    private val signals = new java.util.HashMap[Int, Signal]()
    @jvm("$1:L $2:L invokestatic sun/misc/Signal.handle(Lsun/misc/Signal;Lsun/misc/SignalHandler;)Lsun/misc/SignalHandler;")
    def handle(sig: Signal, handler: SignalHandler): SignalHandler = synchronized {
      java.util.Objects.requireNonNull(sig, "sig")
      java.util.Objects.requireNonNull(handler, "handler")
      val kind =
        if handler eq SignalHandler.SIG_DFL then 0
        else if handler eq SignalHandler.SIG_IGN then 1
        else if handler eq Terminator then 3
        else 2
      val old = signalInstall(sig.getNumber(), kind)
      if old < 0 then throw new IllegalArgumentException("Signal already used by VM or OS: " + sig)
      signals.put(sig.getNumber(), sig)
      val replaced = handlers.remove(sig.getNumber())
      if kind == 2 then handlers.put(sig.getNumber(), handler)
      old match
        case 0 => SignalHandler.SIG_DFL
        case 1 => SignalHandler.SIG_IGN
        case 3 => Terminator
        case _ => replaced
    }
    @jvm("$1:L invokestatic sun/misc/Signal.raise(Lsun/misc/Signal;)V")
    def raise(sig: Signal): Unit = signalRaise(sig.getNumber())
    private[misc] def dispatch(number: Int): Unit =
      val sig = signals.get(number)
      val handler = synchronized(handlers.get(number))
      if handler != null then
        val t = new Thread(() => handler.handle(sig), sig.toString + " handler")
        t.setDaemon(true)
        t.start()
    // The JDK's `Terminator`, the shutdown's handler of INT, TERM and HUP, which `handle` answers for them.
    private object Terminator extends SignalHandler:
      def handle(sig: Signal): Unit = System.exit(sig.getNumber() + 128)

  @jvmClass("sun/misc/SignalHandler")
  trait SignalHandler:
    def handle(sig: Signal): Unit

  @jvmClass("sun/misc/SignalHandler")
  object SignalHandler:
    private val default: SignalHandler = new NativeSignalHandler
    private val ignore: SignalHandler = new NativeSignalHandler
    @jvm("getstatic sun/misc/SignalHandler.SIG_DFL:Lsun/misc/SignalHandler;")
    def SIG_DFL: SignalHandler = default
    @jvm("getstatic sun/misc/SignalHandler.SIG_IGN:Lsun/misc/SignalHandler;")
    def SIG_IGN: SignalHandler = ignore
    private final class NativeSignalHandler extends SignalHandler:
      def handle(sig: Signal): Unit = throw new UnsupportedOperationException("invoking native signal handle not supported")

  // The signal's number on this system, -1 for a name it does not know.
  @js("$fail(\"UnsupportedOperationException\", \"signals are not available on JavaScript\")")
  private[misc] def signalNumber(name: String): Int
  // The handler installed (0 the default, 1 ignored, 2 the program's, 3 the shutdown's): the kind it
  // replaced, -1 for a signal the VM or the system keeps.
  @js("$fail(\"UnsupportedOperationException\", \"signals are not available on JavaScript\")")
  private[misc] def signalInstall(number: Int, kind: Int): Int
  @js("$fail(\"UnsupportedOperationException\", \"signals are not available on JavaScript\")")
  private[misc] def signalRaise(number: Int): Unit

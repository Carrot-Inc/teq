package java.lang:

  // The exception classes of the JVM. In JavaScript `Throwable` extends the native `Error`, so an
  // instance carries a stack trace and passes `instanceof Error`; on the JVM every class here is
  // the JDK's own, and its methods are the JDK's.
  @jvmClass("java/lang/Throwable")
  class Throwable(message: String = null, cause: Throwable = null):
    def this(cause: Throwable) = this(if cause == null then null else cause.toString, cause)
    def this(message: String, cause: Throwable, enableSuppression: Boolean, writableStackTrace: Boolean) = this(message, cause)

    @jvm("invokevirtual java/lang/Throwable.addSuppressed(Ljava/lang/Throwable;)V")
    @javaDefined
    def addSuppressed(other: Throwable): Unit = ()

    @jvm("invokevirtual java/lang/Throwable.getSuppressed()[Ljava/lang/Throwable;")
    @javaDefined
    def getSuppressed: Array[Throwable] = Array()

    @jvm("invokevirtual java/lang/Throwable.getMessage()Ljava/lang/String;")
    @javaDefined
    def getMessage: String = message

    @jvm("invokevirtual java/lang/Throwable.getLocalizedMessage()Ljava/lang/String;")
    @javaDefined
    def getLocalizedMessage: String = getMessage

    @jvm("invokevirtual java/lang/Throwable.getCause()Ljava/lang/Throwable;")
    @javaDefined
    def getCause: Throwable = cause

    // A cause given after construction, once: the instance answers `getCause` with it. A cause
    // passed to a constructor as null counts as none here, where the JDK takes it as given.
    @jvm("invokevirtual java/lang/Throwable.initCause(Ljava/lang/Throwable;)Ljava/lang/Throwable;")
    @javaDefined
    def initCause(c: Throwable): Throwable =
      if getCause != null || causeGiven(this) then
        throw new IllegalStateException("Can't overwrite cause with " + (if c == null then "a null" else c.toString), this)
      if c eq this then throw new IllegalArgumentException("Self-causation not permitted", this)
      giveCause(this, c)
      this

    @jvm("invokevirtual java/lang/Throwable.fillInStackTrace()Ljava/lang/Throwable;")
    @javaDefined
    def fillInStackTrace(): Throwable = this

    @jvm("invokevirtual java/lang/Throwable.printStackTrace()V")
    @javaDefined
    def printStackTrace(): Unit = printStackTraceImpl(this)

    // The frames are not walked on JavaScript: the report is the chain of causes.
    @jvm("invokevirtual java/lang/Throwable.printStackTrace(Ljava/io/PrintWriter;)V")
    @javaDefined
    def printStackTrace(s: java.io.PrintWriter): Unit =
      s.println(this)
      var c = getCause
      while c != null do
        s.println("Caused by: " + c)
        c = c.getCause

    // The stack is not walked: an empty array, where the JVM has the frames, unless the program
    // set one, which JavaScript keeps in a property of the error.
    @js("($0.$stackTrace?.slice() ?? [])")
    @jvm("invokevirtual java/lang/Throwable.getStackTrace()[Ljava/lang/StackTraceElement;")
    @javaDefined
    def getStackTrace: Array[StackTraceElement] = Array()

    @js("void ($0.$stackTrace = $1.slice())")
    @jvm("invokevirtual java/lang/Throwable.setStackTrace([Ljava/lang/StackTraceElement;)V")
    @javaDefined
    def setStackTrace(stackTrace: Array[StackTraceElement]): Unit = ()

    @jvm("invokevirtual java/lang/Throwable.toString()Ljava/lang/String;")
    override def toString: String =
      val name = className
      val text = getLocalizedMessage
      if text == null then name else name + ": " + text

    @js("$0.$qname")
    @jvm("invokevirtual java/lang/Object.getClass()Ljava/lang/Class; invokevirtual java/lang/Class.getName()Ljava/lang/String;")
    private def className: String

  @js("console.error($0.stack !== undefined ? $0.stack : $str($0))")
  def printStackTraceImpl(t: Throwable): Unit

  @js("$giveCause($0, $1)")
  private[lang] def giveCause(t: Throwable, cause: Throwable): Unit

  @js("($0.$causeGiven === true)")
  private[lang] def causeGiven(t: Throwable): Boolean

  @jvmClass("java/lang/Exception")
  class Exception(message: String = null, cause: Throwable = null) extends Throwable(message, cause):
    def this(cause: Throwable) = this(if cause == null then null else cause.toString, cause)
    def this(message: String, cause: Throwable, enableSuppression: Boolean, writableStackTrace: Boolean) = this(message, cause)

  @jvmClass("java/lang/Error")
  class Error(message: String = null, cause: Throwable = null) extends Throwable(message, cause):
    def this(cause: Throwable) = this(if cause == null then null else cause.toString, cause)
    def this(message: String, cause: Throwable, enableSuppression: Boolean, writableStackTrace: Boolean) = this(message, cause)

  @jvmClass("java/lang/RuntimeException")
  class RuntimeException(message: String = null, cause: Throwable = null) extends Exception(message, cause):
    def this(cause: Throwable) = this(if cause == null then null else cause.toString, cause)
    def this(message: String, cause: Throwable, enableSuppression: Boolean, writableStackTrace: Boolean) = this(message, cause)

  @jvmClass("java/lang/IllegalArgumentException")
  class IllegalArgumentException(message: String = null, cause: Throwable = null) extends RuntimeException(message, cause):
    def this(cause: Throwable) = this(if cause == null then null else cause.toString, cause)

  @jvmClass("java/lang/IllegalStateException")
  class IllegalStateException(message: String = null, cause: Throwable = null) extends RuntimeException(message, cause):
    def this(cause: Throwable) = this(if cause == null then null else cause.toString, cause)

  @jvmClass("java/lang/UnsupportedOperationException")
  class UnsupportedOperationException(message: String = null, cause: Throwable = null) extends RuntimeException(message, cause):
    def this(cause: Throwable) = this(if cause == null then null else cause.toString, cause)

  @jvmClass("java/lang/IndexOutOfBoundsException")
  class IndexOutOfBoundsException(message: String = null) extends RuntimeException(message)

  @jvmClass("java/lang/ArrayIndexOutOfBoundsException")
  class ArrayIndexOutOfBoundsException(message: String = null) extends IndexOutOfBoundsException(message)

  @jvmClass("java/lang/StringIndexOutOfBoundsException")
  class StringIndexOutOfBoundsException(message: String = null) extends IndexOutOfBoundsException(message)

  @jvmClass("java/lang/ArithmeticException")
  class ArithmeticException(message: String = null) extends RuntimeException(message)

  @jvmClass("java/lang/NumberFormatException")
  class NumberFormatException(message: String = null) extends IllegalArgumentException(message)

  @jvmClass("java/lang/NullPointerException")
  class NullPointerException(message: String = null) extends RuntimeException(message)

  @jvmClass("java/lang/ClassCastException")
  class ClassCastException(message: String = null) extends RuntimeException(message)

  @jvmClass("java/lang/InterruptedException")
  class InterruptedException(message: String = null) extends Exception(message)

  @jvmClass("java/lang/InstantiationException")
  class InstantiationException(message: String = null) extends Exception(message)

  @jvmClass("java/lang/ReflectiveOperationException")
  class ReflectiveOperationException(message: String = null) extends Exception(message)

  @jvmClass("java/lang/NoSuchMethodException")
  class NoSuchMethodException(message: String = null) extends ReflectiveOperationException(message)

  @jvmClass("java/lang/ClassNotFoundException")
  class ClassNotFoundException(message: String = null, cause: Throwable = null) extends ReflectiveOperationException(message):
    if cause != null then initCause(cause)
    def getException(): Throwable = getCause

  @jvmClass("java/lang/AssertionError")
  class AssertionError(message: String = null, cause: Throwable = null) extends Error(message, cause)

  @jvmClass("java/lang/LinkageError")
  class LinkageError(message: String = null, cause: Throwable = null) extends Error(message, cause)

  @jvmClass("java/lang/IncompatibleClassChangeError")
  class IncompatibleClassChangeError(message: String = null) extends LinkageError(message)

  // What Scala.js's test bridge throws for a framework it cannot find.
  @jvmClass("java/lang/InstantiationError")
  class InstantiationError(message: String = null) extends IncompatibleClassChangeError(message)

  @jvmClass("java/lang/VirtualMachineError")
  class VirtualMachineError(message: String = null, cause: Throwable = null) extends Error(message, cause)

  @jvmClass("java/lang/StackOverflowError")
  class StackOverflowError(message: String = null) extends VirtualMachineError(message)

  @jvmClass("java/lang/OutOfMemoryError")
  class OutOfMemoryError(message: String = null) extends VirtualMachineError(message)

  @jvmClass("java/lang/SecurityException")
  class SecurityException(message: String = null, cause: Throwable = null) extends RuntimeException(message, cause)

package java.util:
  @jvmClass("java/util/NoSuchElementException")
  class NoSuchElementException(message: String = null, cause: Throwable = null) extends RuntimeException(message, cause)

  @jvmClass("java/util/ConcurrentModificationException")
  class ConcurrentModificationException(message: String = null, cause: Throwable = null) extends RuntimeException(message, cause)

// Last in the file: placed before the others, it shifts the ids the classes after it get, and
// `depth_8` took 10% more cycles at the same instructions.
package java.lang:
  @jvmClass("java/lang/NegativeArraySizeException")
  class NegativeArraySizeException(message: String = null) extends RuntimeException(message)

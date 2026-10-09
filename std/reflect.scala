package java.lang:

  // `x.getClass` is a member of `Any` that the compiler types. In JavaScript the class is the JS
  // class of the value, or a stand-in for a primitive, whose prototype carries the names the
  // emitter registers; on the JVM it is the JDK's `Class`.
  @jvmClass("java/lang/Class")
  final class Class[T]:
    @javaDefined
    @js("$0.$qname")
    @jvm("invokevirtual java/lang/Class.getName()Ljava/lang/String;")
    def getName: String

    @javaDefined
    @js("$0.$className")
    @jvm("invokevirtual java/lang/Class.getSimpleName()Ljava/lang/String;")
    def getSimpleName: String

    @javaDefined
    @js("$isInstance($0, $1)")
    @jvm("invokevirtual java/lang/Class.isInstance(Ljava/lang/Object;)Z")
    def isInstance(x: Any): scala.Boolean

    @javaDefined
    @js("(\"class \" + $0.$qname)")
    @jvm("invokevirtual java/lang/Class.toString()Ljava/lang/String;")
    override def toString: String

    @javaDefined
    @js("$isAssignableFrom($0, $1)")
    @jvm("invokevirtual java/lang/Class.isAssignableFrom(Ljava/lang/Class;)Z")
    def isAssignableFrom(other: Class[?]): scala.Boolean

    @javaDefined
    @js("[\"int\", \"long\", \"double\", \"float\", \"short\", \"byte\", \"char\", \"boolean\", \"void\"].includes($0.$qname)")
    @jvm("invokevirtual java/lang/Class.isPrimitive()Z")
    def isPrimitive: scala.Boolean

    @javaDefined
    @js("$0.$qname.startsWith(\"[\")")
    @jvm("invokevirtual java/lang/Class.isArray()Z")
    def isArray: scala.Boolean

    // A public method by name, for a macro that calls a member reflectively: the interpreter
    // looks it up among the members of the class, JavaScript has no reflection.
    @javaDefined
    @js("$fail(\"UnsupportedOperationException\", \"reflection is not available on JavaScript\")")
    def getMethod(name: String, parameterTypes: Class[?]*): java.lang.reflect.Method

    @javaDefined
    @js("$fail(\"UnsupportedOperationException\", \"reflection is not available on JavaScript\")")
    def getMethods: Array[java.lang.reflect.Method]

    // The element class of an array is not kept on JavaScript: `Object` stands for it.
    @javaDefined
    @js("$classOfObject()")
    @jvm("invokevirtual java/lang/Class.getComponentType()Ljava/lang/Class;")
    def getComponentType: Class[?]

  // What `getStackTrace` would hold; nothing makes one, since the stack is not walked.
  final class StackTraceElement(declaringClass: String, methodName: String, fileName: String, lineNumber: Int):
    def getClassName: String = declaringClass
    def getMethodName: String = methodName
    def getFileName: String = fileName
    def getLineNumber: Int = lineNumber
    override def toString: String = s"$declaringClass.$methodName($fileName:$lineNumber)"

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

    // A public field by name, which a class of the program never has (its vals are private
    // fields behind their getters): scala-library's `Selectable.selectDynamic` goes on to the getter.
    @javaDefined
    @js("$fail(\"UnsupportedOperationException\", \"reflection is not available on JavaScript\")")
    def getField(name: String): java.lang.reflect.Field

    // The element class of an array is not kept on JavaScript: `Object` stands for it.
    @javaDefined
    @js("$classOfObject()")
    @jvm("invokevirtual java/lang/Class.getComponentType()Ljava/lang/Class;")
    def getComponentType: Class[?]

    // One loader holds every class on JavaScript and in the interpreter, the bootstrap loader, which
    // the JDK gives as `null`.
    @javaDefined
    @js("null")
    @jvm("invokevirtual java/lang/Class.getClassLoader()Ljava/lang/ClassLoader;")
    def getClassLoader: ClassLoader

  // `forName` finds a class of the program by its binary name, or an array class by its descriptor
  // (`[I`), and initialises an object's class when asked, constructing the instance as the JVM's
  // `<clinit>` does; a primitive's name, a class of the std's or none at all is a
  // `ClassNotFoundException`. The program's classes are kept for it where a call is reached
  // (`$forNameClass`).
  @javaDefined
  @jvmClass("java/lang/Class")
  object Class:
    @js("$forName($1, true)")
    @jvm("invokestatic java/lang/Class.forName(Ljava/lang/String;)Ljava/lang/Class;")
    def forName(className: String): Class[?]
    @js("$forName($1, $2)")
    @jvm("invokestatic java/lang/Class.forName(Ljava/lang/String;ZLjava/lang/ClassLoader;)Ljava/lang/Class;")
    def forName(name: String, initialize: scala.Boolean, loader: ClassLoader): Class[?]

  // What `getStackTrace` would hold; nothing makes one, since the stack is not walked.
  final class StackTraceElement(declaringClass: String, methodName: String, fileName: String, lineNumber: Int):
    def getClassName: String = declaringClass
    def getMethodName: String = methodName
    def getFileName: String = fileName
    def getLineNumber: Int = lineNumber
    // No frame of teq's is a native method's (Scala.js's javalib answers false alike); scala's
    // Boolean, which java.lang's own would shadow here.
    def isNativeMethod: scala.Boolean = false
    override def toString: String = s"$declaringClass.$methodName($fileName:$lineNumber)"

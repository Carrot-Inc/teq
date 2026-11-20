// A constructor run through a method handle is a creation.
import java.lang.invoke.{MethodHandles, MethodType}

class Wide extends jlib.Span(3)

@main def run(): Unit =
  val h = MethodHandles.lookup().findConstructor(classOf[Wide], MethodType.methodType(classOf[Unit]))
  println(h.invoke().asInstanceOf[Wide].n)

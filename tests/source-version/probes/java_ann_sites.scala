@Deprecated("x", false) class A
@Deprecated("x") class B
@SuppressWarnings(Array("x")) class C
@Deprecated(since = "x", forRemoval = false) class D
class E(@Deprecated("p", true) val x: Int) { @SuppressWarnings(Array("a")) def f[@Deprecated("t") T](y: T @Deprecated("u", false)) = y }
@SuppressWarnings(value = Array("a")) class G
object H { def g = { @Deprecated("l", true) val z = 1; z } }

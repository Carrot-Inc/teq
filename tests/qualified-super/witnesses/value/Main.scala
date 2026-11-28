class V(val x: Int) extends AnyVal { def f(x: Any) = super.equals(x) }
@main def run(): Unit = println((new V(1)).f(new V(1)))

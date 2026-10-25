class mark(val n: Int) extends scala.annotation.StaticAnnotation
@main def annotatedType(): Unit = println(Reflect.shape[Int @mark(7)])

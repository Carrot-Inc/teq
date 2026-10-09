// `java.lang.annotation.Annotation` as a parent of a class that is no annotation, as munit's
// `Location` has it.
final class Tag(val name: String) extends java.lang.annotation.Annotation:
  def annotationType(): Class[java.lang.annotation.Annotation] = classOf[java.lang.annotation.Annotation]
  override def toString: String = s"Tag($name)"

@main def run(): Unit =
  val t = new Tag("x")
  println(t)
  println(t.isInstanceOf[java.lang.annotation.Annotation])
  val a: java.lang.annotation.Annotation = t
  println(a.annotationType() == classOf[java.lang.annotation.Annotation])

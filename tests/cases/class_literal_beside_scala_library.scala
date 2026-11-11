// jars: scala-library
// Class literals with scala-library's jar on the class path beside teq's std: `classOf` is the std's own
// (`Definitions.Predef_classOf`), whose application is the literal, and not the jar's `Predef.classOf`.
@main def run(): Unit =
  println(classOf[Object].getName)
  println(classOf[AnyRef].getName)
  println(classOf[String].getName)
  println(classOf[java.lang.Integer].getName)

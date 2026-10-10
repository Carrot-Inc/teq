@deprecated("old annotation", "1") class Old extends scala.annotation.StaticAnnotation
@Old class Fresh
object Main { def main(args: Array[String]): Unit = println("ok") }

import scala.annotation.nowarn
object Lib {
 @deprecated("old annotation", "1") class Old extends scala.annotation.StaticAnnotation
 @nowarn("cat=deprecation") type Alias = Old
}
@Lib.Alias class Fresh
object Main { def main(args: Array[String]): Unit = println("ok") }

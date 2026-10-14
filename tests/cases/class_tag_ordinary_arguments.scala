// A standard `ClassTag` passed as an ordinary argument of a member that takes evidence too (an array of tags
// `appended` or `prepended` one) is the element itself: only the evidence parameters' standard tags are the ones
// the JavaScript output may pass as `null`, as scala-library's `ArrayOps.appended` keeps the element supplied.
import scala.reflect.ClassTag
object Main:
 def main(args: Array[String]): Unit =
  println(Array(ClassTag.Int).toList)
  println(Array.empty[ClassTag[Int]].appended(ClassTag.Int).toList)
  println(Array.empty[ClassTag[Int]].prepended(ClassTag.Int).toList)

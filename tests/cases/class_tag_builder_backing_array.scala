// ArrayBuilder.ofRef writes each element into the backing array its tag allocated as it is added and
// grows it by copying (`addOne`, `resize`): a callback reading that array sees the elements before it,
// and after a throwing callback it holds them (the review's `BuilderState`).
import scala.reflect.ClassTag
object Main:
 var dest: Array[String] = Array.empty[String]
 val tag = new ClassTag[String]:
  def runtimeClass = classOf[String]
  override def newArray(n: Int): Array[String] =
   dest = new Array[String](n)
   dest
 def main(args: Array[String]): Unit =
  val result = Array(1, 2).collect { case i =>
   if i == 2 then println("seen: " + dest(0))
   "v" + i
  }(using tag)
  println(result.toList)
  println(dest.take(2).toList)
  try
   Array(1, 2, 3).collect { case i =>
    if i == 2 then throw new RuntimeException("stop")
    "v" + i
   }(using tag)
  catch case _: RuntimeException => println(dest.take(3).toList)
  try
   Array.tabulate[String](3)(i => if i == 1 then throw new RuntimeException("stop") else "v" + i)(using tag)
  catch case _: RuntimeException => println(dest.toList)

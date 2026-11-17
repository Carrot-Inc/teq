// A program's own annotation classes named `js` and `jvmEvidence`, on a method of the program whose
// text names its class tag under a condition: the tag is evaluated once at every call, ahead of
// the text, and the condition reads the tag itself.
import scala.reflect.ClassTag

class js(val text: String) extends scala.annotation.StaticAnnotation
class jvmEvidence extends scala.annotation.StaticAnnotation

@jvmEvidence
@js("($0 ? $1 !== null : true)")
def probe(b: Boolean)(using tag: ClassTag[Int]): Boolean =
  if b then tag != null else true

def loud: ClassTag[Int] = { println("tag"); ClassTag.Int }

object Main {
  def main(args: Array[String]): Unit = {
    println(ClassTag.Int != null)
    println(probe(false)(using ClassTag.Int))
    println(probe(true)(using ClassTag.Int))
    println(probe(false)(using loud))
    println(probe(true)(using loud))
  }
}

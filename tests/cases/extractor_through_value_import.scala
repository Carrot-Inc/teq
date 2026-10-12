// An extractor object of a class, imported through a value of the class: its `unapply`'s parameter
// is seen from the value (dotty's typedUnApply reads `qual.unapply`'s type), `tasty.Select` here,
// and a wider scrutinee is tested through the ClassTag the value gives.
import scala.reflect.ClassTag

trait Tasty:
  type Term
  type Select <: Term
  given selectTag: ClassTag[Select]
  object Select:
    def unapply(x: Select): Option[String] = Some(x.toString.reverse)

class StringTasty extends Tasty:
  type Term = Any
  type Select = String
  given selectTag: ClassTag[Select] = ClassTag(classOf[String])

def classify(using tasty: Tasty)(t: tasty.Term): String =
  import tasty.{*, given}
  t match
    case Select(s) => s"select $s"
    case _ => "other"

def show(using tasty: Tasty)(t: tasty.Select): String =
  import tasty.*
  t match
    case Select(s) => s"exact $s"

@main def main(): Unit =
  given t: StringTasty = StringTasty()
  println(classify("abc": t.Term))
  println(classify(5: t.Term))
  println(show("xyz": t.Select))

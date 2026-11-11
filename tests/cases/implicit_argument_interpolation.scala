trait Mark[A]
class Evidence[A, B]
object Evidence:
  given [A <: Mark[?], B <: Mark[?]]: Evidence[A, B] = new Evidence[A, B]
class Layer[+A]:
  def combine[B >: A, C](next: Layer[C])(using Evidence[B, C]): Layer[B & C] = ???
class First
class Second
class Third
def compose(a: Layer[Mark[First]], b: Layer[Mark[Second]], c: Layer[Mark[Third]]): Layer[Mark[First] & Mark[Second] & Mark[Third]] =
  a.combine(b).combine(c)

trait Key:
  type Value
object Key:
  type IntKey = Key { type Value = Int }
  given key: IntKey = new Key { type Value = Int }
  given payload: key.Value = 7
def dependent(using key: Key, value: key.Value): key.Value = value
@main def run(): Unit =
  import Key.given
  val result: Key.key.Value = dependent
  println(result)

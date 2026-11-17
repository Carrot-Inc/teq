// expect: 37:18: error: missing argument list for value of type Int => Int
// expect: 38:19: error: missing argument list for value of type Int => Int
// expect: 39:18: error: missing argument list for value of type Int => Int
// expect: 40:18: error: missing argument list for value of type String => String
// expect: 41:18: error: missing argument list for value of type Int => Int
// expect: 42:18: error: method tick in object Counts must be called with () argument
// expect: 46:30: error: missing argument list for value of type Int => Int
// expect: 48:20: error: missing argument list for value of type Int => Int
// expect: 52:18: error: missing argument list for value of type Int => Int
// expect: 53:18: error: missing argument list for value of type String => String
// expect: 54:18: error: missing argument list for value of type Int => Int
// expect: 55:22: error: missing argument list for value of type String => Tone
// absent: 42:18: error: missing argument list
// absent: 43:18: error
// absent: 44:18: error
// absent: 45:18: error
// absent: 47:30: error
// absent: 49:20: error
// absent: 50:20: error
// A method's name is refused where `Unit` is wanted only where the name itself is written there (alone, in
// parentheses, before a trailing ` _`, short of a list, selected): the closure the typer made of it at that
// place. A function value that an inline call's expansion or an applied literal leaves there is no such closure,
// also where a method's name was expanded inside it; a method of an empty list named bare is told to want its
// `()` and nothing else. The closure of a polymorphic function value named with its type, of an extension method
// named through its object, of an enum companion's lookup and the `apply` selected of a method's name are refused
// alike.
enum Tone:
  case Low, High
object Texts:
  extension (s: String) def doubled: String = s + s
object Counts:
  def twice(i: Int): Int = i * 2
  def join(a: String)(b: String): String = a + b
  def tick(): Int = 1
  transparent inline def handed(inline f: Int => Int): Int => Int = f
  transparent inline def named: Int => Int = twice
  def a1: Unit = twice
  def a2: Unit = (twice)
  def a3: Unit = twice _
  def a4: Unit = join("a")
  def a5: Unit = Counts.twice
  def a6: Unit = tick
  def b1: Unit = handed(twice)
  def b2: Unit = named
  def b3: Unit = ((i: Int) => twice)(1)
  val c1: () => Unit = () => twice
  val c2: () => Unit = () => named
  def d1: Unit = { twice; () }
  def d2: Unit = { named; () }
  def d3: Unit = { handed(twice); () }
  val poly: [A] => A => A = [A] => (a: A) => a
  def e1: Unit = poly[Int]
  def e2: Unit = Texts.doubled
  def e3: Unit = twice.apply
  def e4(): Unit = { Tone.valueOf; () }

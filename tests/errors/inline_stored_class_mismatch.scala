// expect: 10:23: error: type mismatch: found Show, required Int
// expect: 13:3: error: type mismatch: found Show, required Int
// expect: 16:3: error: type mismatch: found L, required String
// expect: 3 errors found
// The classes the definition check keeps with the record take none of the output's names: a
// message of the check that shows such a class shows its parents or its own name, as scalac's
// shows them (`Object with Show {...}`, `(s : Show)`, `L`), never a name of the record's.
trait Show:
  def show(a: Int): String
inline def bad: Int = new Show { def show(a: Int) = "x" }
inline def bad2: Int =
  val s = new Show { def show(a: Int) = "y" }
  s
inline def bad3: String =
  class L extends Show { def show(a: Int) = "z" }
  new L
@main def run(): Unit = println(bad + bad2 + bad3)

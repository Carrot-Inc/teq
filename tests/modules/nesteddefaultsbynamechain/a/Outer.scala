package ndbc

// By-name defaults in a chain of clauses: the second reads the first twice, the third the second twice.
// Each getter takes the earlier arguments as the trees the creation passed (`Applications.spliceMeth`),
// selected on the companion of the creation's prefix (`Applications.findDefaultGetter`), so every read
// evaluates the first default again, as the class's own reads of its by-name parameter do.
class Outer(val n: Int) {
  var count = 0
  def read(): Int = { count += 1; println("read " + count); n + count }
  class Item(a: => Int = read())(b: => Int = a + a)(val c: Int = b + b) {
    def get = b
  }
}

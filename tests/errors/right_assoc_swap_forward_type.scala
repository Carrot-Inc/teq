// The receiver of a right-associative extension cannot name the method's own type parameter.
object Lib:
  extension (x: A)
    def %:[A](y: A): A = y
@main def run(): Unit = println("ok")
// expect: 3:17: error: right-associative extension method cannot have a forward reference to A

object PrototypeDiagnostics {
  def unary(f: Int => Int): Int = 0
  def unary(f: String => String): String = ""
  def binary(f: (Int, String) => Int): Int = 0
  def binary(f: (Int, Boolean) => Int): String = ""
  def cases(f: PartialFunction[Int, Int]): Int = 0
  def cases(f: PartialFunction[String, String]): String = ""
  def check(): Unit = {
    unary(x => x)
    binary((x: Int, y) => x)
    cases { case x => x }
  }
}
// Scalac 3.8.4: unforced argument prototypes retain unknown parameter and result types.
// expect: 9:5: error: Ambiguous overload. The overloaded alternatives of method unary in object PrototypeDiagnostics with types
// expect:  (f: String => String): String
// expect:  (f: Int => Int): Int
// expect: both match arguments (<?> => <?>)
// expect: 10:5: error: Ambiguous overload. The overloaded alternatives of method binary in object PrototypeDiagnostics with types
// expect:  (f: (Int, Boolean) => Int): String
// expect:  (f: (Int, String) => Int): Int
// expect: both match arguments ((Int, <?>) => <?>)
// expect: 11:5: error: Ambiguous overload. The overloaded alternatives of method cases in object PrototypeDiagnostics with types
// expect:  (f: PartialFunction[String, String]): String
// expect:  (f: PartialFunction[Int, Int]): Int
// expect: both match arguments (<?>)
// expect: 3 errors found

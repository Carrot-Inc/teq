// A `case` inside parentheses opens no case clause there: refused, as scalac refuses it.
// expect: 4:27: error: expected an expression, found 'case'
@main def run(): Unit =
  println(List(1).collect(case 1 |

    2 => 7))

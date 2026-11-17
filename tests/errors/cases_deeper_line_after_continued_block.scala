// expect: 18:3: error: expected end of statement, found 'match'
// expect: 27:3: error: expected end of statement, found 'match'
// expect: 35:3: error: expected end of statement, found 'match'
// expect: 44:3: error: expected end of statement, found identifier
// expect: 51:43: error: expected end of statement, found identifier
// What continues a block, then a `match` with its cases at its own width, then a line further in than the cases:
// refused at the token that continues the block: the `match` after an assignment's indented right side, after the
// indented `else` branch of an `inline if`, and after the block in braces that is the body of a `catch`'s case; the
// operator where one continues a case's block before the `match`, on the next line and on the case's line. scalac
// reads the line further in as the last case's second statement and accepts each program
// (tests/pending-cases/layout_cases_deeper_line_after_block, _after_inline_else, _after_case_block).
object Q:
  var slot: Any = "0"
  var seen = ""
  def t: Any =
    slot =
        "a"
  match
  case o if seen == "x" => "mx"
  case o => seen += "m"
    slot

inline def branch(inline c: Boolean): Any =
  inline if c then "t"
  else
      "a"
  match
  case o if Q.seen == "x" => "mx"
  case o => Q.seen += "m"
    "zz"

def caught(x: String): Any =
  try throw new RuntimeException("r")
  catch case e: RuntimeException => { x }
  match
  case o if o == "x" => "mx"
  case o => Q.seen += "m"
    "zz"

def afterOperator(n: Int, w: String): Any =
  n match
  case 1 => "one"
  case _ => { w }
  + "z" match
  case o if o == "xz" => "mx"
  case o => Q.seen += "m"
    "zz"

def caughtOperator(w: String): Any =
  try throw new RuntimeException("r")
  catch case e: RuntimeException => { w } + "z"
  match
  case o if o == "xz" => "mx"
  case o => Q.seen += "m"
    "zz"

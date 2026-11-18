// expect: 10:3: error: The start of this line does not match any of the previous indentation widths.
// expect: Indentation width of current line : 2 spaces
// expect: This falls between previous widths: 0 spaces and 4 spaces
// A line that stands between two indentation widths and begins with an operator, the operator's operand on the
// next line and further left: the line is reported with its own width and the two widths around it. scalac: the
// same three lines, at the operator.
def f: Int =
    val a = 1
    a
  +
1

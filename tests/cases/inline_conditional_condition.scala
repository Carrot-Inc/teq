// An inline conditional as another conditional's condition: its expansion as the call's own tree, as an inline
// parameter's, under an ascription and as an operand. scalac prints the lines of the .expected file.
inline def choose(a: Boolean, b: Boolean, c: Boolean): Boolean = if a then b else c
inline def chooseInline(inline a: Boolean, inline b: Boolean, inline c: Boolean): Boolean = if a then b else c
def f(a: Boolean, b: Boolean, c: Boolean): Int = if choose(a, b, c) then 10 else 20
def g(a: Boolean, b: Boolean, c: Boolean): Int = if chooseInline(a, b, c) then 10 else 20
def h(a: Boolean, b: Boolean, c: Boolean): Int = if (choose(a, b, c): Boolean) then 10 else 20
def k(a: Boolean, b: Boolean, c: Boolean): Boolean = choose(a, b, c) && !choose(c, a, b)
def m(a: Boolean, b: Boolean, c: Boolean): Int = (if choose(a, b, c) then 1 else 2) + (if chooseInline(c, b, a) then 4 else 8)
@main def run(): Unit =
  for (a, b, c) <- List((true, false, false), (true, true, false), (false, false, true), (false, true, false)) do
    println(s"${f(a, b, c)} ${g(a, b, c)} ${h(a, b, c)} ${k(a, b, c)} ${m(a, b, c)}")

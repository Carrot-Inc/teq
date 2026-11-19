trait Need[A]
type F = Need[Int] ?=> Int => Int
def use(f: F): Int = f(1)
// A context function applied: scalac 3.8.4 names its type as written, the alias (`parameter of F`), and says what it
// is an alias of on a line of its own (Implicits.implicitParamString's `Where F is an alias of:`); teq prints the
// alias's expansion (a type that remembers its alias: the printer's own row). scalac's place is one column on (`(1)`).
// expect: 3:22: error: No given instance of type Need[Int] was found for parameter of F

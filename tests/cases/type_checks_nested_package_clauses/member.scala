// `typeChecks` types its code in the file it is called from, nested in two package clauses: the outer package's
// `println(Int)` comes before `Predef`'s root import, so `println("s")` does not type and `println(2)` does, as
// scalac answers.
package a
def println(x: Int): Unit = System.out.println("mine " + x)

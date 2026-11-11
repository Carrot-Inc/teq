// A file of package `scala` nested in a package of its own reaches package `scala`'s members, as its enclosing
// package clause's, before `Predef`'s root import, which encloses the package clauses: `println(1)` is the
// package's `println`, as scalac resolves it.
package scala
def println(x: Any): Unit = System.out.println("mine " + x)

// The proxy of a `@main` method of one package of a file names nothing of another: a wildcard import
// of that other package is unused.
package r {
  import r.*
  class C
}
package s {
  @main def go(): Unit = println(3)
}

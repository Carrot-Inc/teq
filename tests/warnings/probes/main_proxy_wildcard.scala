// A top-level `@main` method's proxy class calls it from outside the file's package object, where a
// wildcard import of the method's package brings it: the import is used.
package p
import p.*
@main def run(): Unit = println(1)

// A top-level `@main` method's proxy class calls it, from outside the file's package object, by its
// name and the package as its prefix (`MainProxies.mainProxy`): an import of the method's own
// package that brings it is used, a named one before a wildcard.
package p
import p.*
import p.run
@main def run(): Unit = println(1)

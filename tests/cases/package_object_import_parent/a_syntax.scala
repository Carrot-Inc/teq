package scalaql.syntax

import scalaql.*

// The package object's parent, which the package object's file imports through the package
// itself: reading the package's members while that object completes passes over what it
// inherits (dotty's `PackageClassDenotation.computeMembersNamed`, a package object
// `isCompleting`), as scala3's tests/pos/i15980 has it, its files in that test's order (scalac
// meets a cyclic reference in another).
@forbiddenInheritance
trait ScalaqlSyntax:
  def hello: String = "hello from the package object's parent"

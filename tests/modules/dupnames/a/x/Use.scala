package dna

// A file of the same name in each module: under the case's root their keys differ
// (a/x/Use.scala, b/x/Use.scala), so do their tokens and the names made of them.
trait Named:
  def name: String

object Use:
  def make: Named = new Named:
    def name = "a"

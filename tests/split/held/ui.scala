// Under --module-per-file ui,views --hot: what decides whether a per-file module can be swapped.
// Shared code (package logic) tests a class of `ui` and compares a value of its enum, so a value
// the page holds from before a swap of `ui` would stop matching there: `ui` reloads the page
// when it is run again. `Token` is for the comparisons with an object that shared code makes. Of `views` shared code constructs a class, calls a def and reads a val,
// all of which take what a swap made: `views` is swapped.
package ui

case class Selection(index: Int)

enum Mark:
  case Red, Green

def first: Selection = Selection(1)

object Token:
  def label: String = "token"

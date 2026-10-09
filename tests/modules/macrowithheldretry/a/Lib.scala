package mra

// A body the lean products withhold (`List.range`, which the Scala.js check build states under
// another shape than scala-library's), which an interpolator's macro of this module reaches at its
// expansion downstream.
object Addresses:
  def valid(s: String): Boolean = s.length > 2 && List.range(0, s.length).exists(i => s.charAt(i) == '@')

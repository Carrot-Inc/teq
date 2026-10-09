package mha

// An ordinary helper of the upstream, which a macro of the downstream calls at its expansion.
object Help:
  def label(n: Int): String = "n=" + (n * 2)

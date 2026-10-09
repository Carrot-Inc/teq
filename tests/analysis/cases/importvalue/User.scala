package p

// File imports nothing uses, through a stable value (`p` is the package's val `p.p`, not the
// package `p`) and through an earlier import's binding (`Inner` of `Impl.Inner`).
import p.n
import Impl.Inner
import Inner.k

object User:
  def f: Int = 0

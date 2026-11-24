// A cycle of constants through two files: each inferred from the other, refused as scalac refuses it.
object A { final val a = if (B.y) 1 else 2; inline val x: a.type = a }
// expect: 2:58: error: Cyclic reference involving val a

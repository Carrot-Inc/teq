// jars: scala-library cats-kernel cats-core cats-free cats-effect-kernel-jvm cats-effect-std-jvm cats-effect-jvm fs2-core-jvm fs2-io scodec-bits scodec-core scodec-cats cats-parse ip4s-core literally natchez-core sourcepos typename twiddles-core skunk-core
// skunk's `sql` interpolator from its jar, a transparent macro: the expansion's static type is
// the fragment of the encoders' types, `Fragment[Int *: Int *: EmptyTuple]` for two encoders
// folded with twiddles' `*:` inside a block a pickled quote holds.
import skunk.*
import skunk.implicits.*
import skunk.codec.all.*

object Q:
  def none: Fragment[Void] = sql"select 1"
  def one: Fragment[Int] = sql"select a from t where id = $int4"
  def two: Fragment[Int *: Int *: EmptyTuple] = sql"a $int4 and $int4"
  def three: Fragment[(Int, String, Int)] = sql"a $int4 b $text c $int4"

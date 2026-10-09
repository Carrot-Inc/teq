package app.util

import sharedlib.tailwind.TailwindSyntax.tw

object Styles:
  val card: sharedlib.tailwind.Tw = tw"rounded shadow"

  def spacing(n: Int): sharedlib.tailwind.Tw = tw"p-$n m-${n * 2}"

object Prelude extends sharedlib.PreludeCore:
  export Styles.{card, spacing as pad}

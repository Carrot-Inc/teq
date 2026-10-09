package util

object DeskAppUtils:
  enum ViewVariant:
    case Desktop, Mobile, Tablet

  export ViewVariant.{Desktop, Mobile}

  def describe(v: ViewVariant): String = v match
    case Desktop => "desktop"
    case Mobile => "mobile"
    case ViewVariant.Tablet => "tablet"

  given Ordering[ViewVariant] with
    def compare(a: ViewVariant, b: ViewVariant): Int = b.ordinal - a.ordinal

object Prelude extends sharedlib.PreludeCore:
  export util.DeskAppUtils.{*, given}
  export facade.IconSet
  export facade.IconSet.IconDef
  export sharedlib.Utils.{clamp as clampInt, defaultLimit as _}

  def smallIcon(name: String): IconDef = IconDef(name, 8)

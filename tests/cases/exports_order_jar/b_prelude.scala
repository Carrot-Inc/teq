package util

import sharedlib.PreludeCore

object Prelude extends PreludeCore:
  export cats.data.NonEmptyList.{of => nel, one => single}

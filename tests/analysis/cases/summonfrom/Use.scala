package selection

// The given a summonFrom finds for a case that binds none (`_: Marker`), which scalac's
// expansion binds (`case _$1: Marker`) and its dependencies name.
class Use:
  def value: Int = selected

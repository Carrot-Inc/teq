package vaa

// `@volatile` under a type alias (`type V = scala.volatile`) marks the field as the annotation does, in the whole
// build as over the products, which the classes check compares byte for byte (the whole build's flag was once
// missing, the split build's set).
type V = scala.volatile

trait Flags:
  @V var stop: Boolean = false
  var plain: Int = 1

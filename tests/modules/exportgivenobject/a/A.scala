package goa

trait Tag:
  def value: Int

object A:
  given tag: Tag with
    def value: Int = 7

// A given object's forwarders are stable, their results its singleton `A.tag.type`: the same
// given through two imports is no ambiguity.
object B:
  export A.tag

object C:
  export A.{tag as another}

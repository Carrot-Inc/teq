package react

class ReactEvent(val kind: String)

trait ReactEventTypes:
  def event(kind: String): ReactEvent = ReactEvent(kind)

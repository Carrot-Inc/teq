package gamma

// A source of gamma, which builds on alpha: its classpath holds alpha's products.
object Hello:
  def greeting: String = shared.Shared.greeting + " through gamma"

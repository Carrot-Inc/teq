package web

@main def run(): Unit =
  println(Page.render() + Calls.top() + Page.area(shapes.Circle(1.0)))

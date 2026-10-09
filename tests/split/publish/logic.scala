package logic

// A class, so that the runtime's helpers for classes are in every build of the scenarios.
class Line(val text: String)

def line(name: String): String = Line(parts.greet(name) + "!").text

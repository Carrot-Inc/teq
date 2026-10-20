// lila: web/PrometheusReporter.scala:13 (`class PrometheusReporter(configPath: String = DefaultConfigPath, ...)`)
// A constructor's default argument is typed with the companion object's members in scope.
class Reporter(configPath: String = DefaultConfigPath):
  def path = configPath

object Reporter:
  final val DefaultConfigPath = "kamon.prometheus"

@main def run() = println(Reporter().path)

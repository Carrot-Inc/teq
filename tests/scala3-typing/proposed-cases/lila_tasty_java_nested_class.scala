//> using dep org.playframework::play-ahc-ws-standalone:3.0.14
// lila: app/Lila.scala:76 (`AhcConfigBuilder(...).modifyUnderlying(_.setIoThreadsCount(8))`)
// Needs the library: modifyUnderlying's TASTy signature names the Java static nested class
// play.shaded.ahc...DefaultAsyncHttpClientConfig.Builder, whose Java methods are members of the
// lambda parameter. teq: --classpath with play-ahc-ws-standalone_3, play-ws-standalone_3,
// shaded-asynchttpclient, ssl-config-core_3, config, slf4j-api (lila's jars).
import play.api.libs.ws.ahc.{ AhcConfigBuilder, AhcWSClientConfig }

@main def run() =
  val cfg = AhcConfigBuilder(AhcWSClientConfig()).modifyUnderlying(_.setIoThreadsCount(8)).build()
  println(cfg.getIoThreadsCount)

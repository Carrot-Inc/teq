// The oracle's downstream for cc.scala (README.md), read with capture checking enabled, under which dotty keeps a
// retaining annotation it reads (RetainingAnnotation.mapWith, `Feature.ccEnabledSomewhere`); both payloads of
// ANNOTATEDtype, the tree of -source:3.8 and the type of -source:3.9, read as one compact annotation
// (TreeUnpickler.scala 427-433).
import language.experimental.captureChecking

@main def observeCc(): Unit = println(observe.Observe.members("object:repr.cc.Cap"))

// A type member of a trait's self type is path-dependent through `this`: inside the trait its
// alias holds, outside a receiver that is not known to be the self type cannot resolve it
// (scala3's neg/i8405 and neg/i11226a).
// expect: 14:49: error: type mismatch: found ActorRef, required (Unsubscriber.this.bus : ManagedActorClassification).Subscriber (which cannot be resolved: ManagedActorClassification has no member Subscriber)
// expect: 1 error found
class ActorRef
trait ActorEventBus:
  type Subscriber = ActorRef
trait ManagedActorClassification:
  this: ActorEventBus =>
  def unsubscribe(subscriber: Subscriber): Unit = ()
  def unsubscribeSelf(a: ActorRef): Unit = unsubscribe(a)
class Unsubscriber(bus: ManagedActorClassification):
  def test(a: ActorRef): Unit = bus.unsubscribe(a)
@main def run(): Unit = ()

# ltrender

powerful rendering engine for terminal tools and games.


## TODO:
- documentation
- more examples
## Improve:
- add more testing

## Remember:
- remember to make the renderers interval expansion happen, after the diffing has been done, so intervals, that have not change aren diffed


## reimplement this function
/// used to check every 5 ms, if an object needs to be removed from screen
/// extits if the receiver is gone
/// TODO: make this code smarter
fn run_object_lifetime_checker(
    tx: SyncSender<RendererCommand>,
    shutdown: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(5));
            if tx
                .send(RendererCommand::CheckIfObjectLifetimeEnded())
                .is_err()
            {
                break;
            }
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
        }
    })
}
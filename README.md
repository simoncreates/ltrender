# ltrender

powerful rendering engine for terminal tools and games.


## TODO:
- add proper testing
- seperation of concerns: 
   the concept of buffered and instant rendering needs to be seperate and clear

## Remember:
- find a way how it can be regularly checked, if an object with a timed lifetime has "expired"
- replace all the object adding and removing logic to be compatible with the new lifetime system
- make sure that screens only hold objects, whos lifetimes have not expired
   and make sure, that the ones who are on screen, will be rendered unconditionally
export default class MovingWindowEstimator {
  private readonly samples: (number | null)[] = [];

  public constructor(private readonly window: number) {}

  public addSample(value: number | null) {
    this.samples.push(value);

    while (this.samples.length > this.window) {
      this.samples.shift();
    }
  }

  public minimum(): number | null {
    let minimum = Number.POSITIVE_INFINITY;

    for (const sample of this.samples) {
      if (sample === null) {
        return null;
      }

      minimum = Math.min(minimum, sample);
    }

    return minimum;
  }

  public average(): number | null {
    let average = 0.0;

    for (const sample of this.samples) {
      if (sample === null) {
        return null;
      }

      average += sample;
    }

    return average / this.samples.length;
  }
}

const glowRadius = 0.9;
const gradientSteps = 2_048;
const falloff = new Float64Array(gradientSteps + 2);

// The bright center and the softer outer glow fade together on one continuous curve.
for (let step = 0; step < gradientSteps; step++) {
  const distance = step / gradientSteps;
  const centerDistance = 0.7 * distance;
  const center = 0.39 * Math.exp(-4 * centerDistance - 32 * centerDistance ** 2);
  const outer = 0.23 * (1 - distance) ** 1.15;
  falloff[step] = center + outer;
}

function channels(hex: string) {
  const color = Number.parseInt(hex.slice(1), 16);
  return [color >> 16, (color >> 8) & 255, color & 255];
}

function drawGradient(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  baseColor: string,
  indicatorColor: string,
) {
  const image = context.createImageData(width, height);
  const [red, green, blue] = channels(baseColor);
  const [accentRed, accentGreen, accentBlue] = channels(indicatorColor);
  const redDelta = accentRed - red;
  const greenDelta = accentGreen - green;
  const blueDelta = accentBlue - blue;
  const radialScale = gradientSteps / (glowRadius * Math.hypot(width, height));
  const xSquared = Float64Array.from({ length: width }, (_, x) => (x + 0.5) ** 2);
  let offset = 0;
  let noise = 1;

  for (let y = 0; y < height; y++) {
    const ySquared = (y + 0.5) ** 2;
    for (let x = 0; x < width; x++) {
      const sample = Math.min(gradientSteps, Math.sqrt(xSquared[x] + ySquared) * radialScale);
      const step = Math.floor(sample);
      const tint = falloff[step] + (falloff[step + 1] - falloff[step]) * (sample - step);

      // Stochastically round the fractional color once, using one threshold for all channels.
      noise ^= noise << 13;
      noise ^= noise >>> 17;
      noise ^= noise << 5;
      const threshold = (noise >>> 24) / 256;
      image.data[offset++] = Math.floor(red + redDelta * tint + threshold);
      image.data[offset++] = Math.floor(green + greenDelta * tint + threshold);
      image.data[offset++] = Math.floor(blue + blueDelta * tint + threshold);
      image.data[offset++] = 255;
    }
  }
  context.putImageData(image, 0, 0);
}

export function focusBackground(canvas: HTMLCanvasElement, _contextStyle: string) {
  const context = canvas.getContext('2d');
  if (!context) return;

  let width = 0;
  let height = 0;
  let frame: number | null = null;
  let painted = '';

  const schedule = () => {
    if (!width || !height || frame !== null) return;
    frame = requestAnimationFrame(() => {
      frame = null;
      const style = getComputedStyle(canvas);
      const base = style.getPropertyValue('--surface-sunken').trim();
      const indicator = style.getPropertyValue('--context-indicator').trim();
      const key = `${width}:${height}:${base}:${indicator}`;
      if (key === painted) return;
      if (canvas.width !== width) canvas.width = width;
      if (canvas.height !== height) canvas.height = height;
      drawGradient(context, width, height, base, indicator);
      painted = key;
    });
  };

  const resize = new ResizeObserver(([{ contentRect }]) => {
    width = Math.round(contentRect.width * devicePixelRatio);
    height = Math.round(contentRect.height * devicePixelRatio);
    schedule();
  });
  resize.observe(canvas, { box: 'device-pixel-content-box' });

  return {
    update: schedule,
    destroy() {
      resize.disconnect();
      if (frame !== null) cancelAnimationFrame(frame);
    },
  };
}

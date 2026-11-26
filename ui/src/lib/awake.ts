// Keeps the phone's screen on.
//
// The Screen Wake Lock API only exists on secure pages. Over USB the dashboard
// is `localhost`, which counts; over Wi-Fi it is plain http on a local address,
// which does not. There the old trick still works: a tiny silent video that
// plays in a loop keeps the display awake.

/** Four seconds of black, 16×16, with silent audio (a few kB of WebM). */
const CLIP =
  'data:video/webm;base64,' +
  [
  'GkXfo59ChoEBQveBAULygQRC84EIQoKEd2VibUKHgQRChYECGFOAZwEAAAAAAAnbEU2bdLpNu4tTq4QVSalmU6yBoU27i1OrhBZUrmtTrIHWTbuMU6uEElTD',
  'Z1OsggGaTbuMU6uEHFO7a1OsggnF7AEAAAAAAABZAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
  'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAVSalmsCrXsYMPQkBNgIxMYXZmNjMuMS4xMDJXQYxMYXZmNjMuMS4xMDJEiYhAr1AAAAAAABZUrmtAvq4B',
  'AAAAAAAARNeBAXPFiJcj1NaeuZvAnIEAIrWcg3VuZIiBAIaFVl9WUDiDgQEj44OEO5rKAOCGsIEQuoEQVe6BAOwBAAAAAAAAAgAArgEAAAAAAABo14ECc8WI',
  'BPx7mTIycNScgQAitZyDdW5kiIEAhoZBX09QVVNWqoNjLqBWu4QExLQAg4ECI+ODhAExLQDhkZ+BAbWIQOdwAAAAAABiZIEQVe6BAGOik09wdXNIZWFkAQE4',
  'AYC7AAAAAAASVMNnQIxzc59jwIBnyJlFo4dFTkNPREVSRIeMTGF2ZjYzLjEuMTAyc3OyY8CLY8WIlyPU1p65m8BnyKFFo4hEVVJBVElPTkSHkzAwOjAwOjA0',
  'LjAwMDAwMDAwMABzc7JjwItjxYgE/HuZMjJw1GfIoUWjiERVUkFUSU9ORIeTMDA6MDA6MDQuMDA4MDAwMDAwAB9DtnVHk+eBAKOHggAAgPj//qOjgQAAgBAC',
  'AJ0BKhAAEAAARwiFhYiZhIgCAgAMDWAA/v+rUICjh4IAFYD4//6jh4IAKYD4//6jh4IAPYD4//6jh4IAUYD4//6jh4IAZYD4//6jh4IAeYD4//6jh4IAjYD4',
  '//6jh4IAoYD4//6jh4IAtYD4//6jh4IAyYD4//6jh4IA3YD4//6jh4IA8YD4//6jh4IBBYD4//6jh4IBGYD4//6jh4IBLYD4//6jh4IBQYD4//6jh4IBVYD4',
  '//6jh4IBaYD4//6jh4IBfYD4//6jh4IBkYD4//6jh4IBpYD4//6jh4IBuYD4//6jh4IBzYD4//6jh4IB4YD4//6jh4IB9YD4//6jh4ICCYD4//6jh4ICHYD4',
  '//6jh4ICMYD4//6jh4ICRYD4//6jh4ICWYD4//6jh4ICbYD4//6jh4ICgYD4//6jh4IClYD4//6jh4ICqYD4//6jh4ICvYD4//6jh4IC0YD4//6jh4IC5YD4',
  '//6jh4IC+YD4//6jh4IDDYD4//6jh4IDIYD4//6jh4IDNYD4//6jh4IDSYD4//6jh4IDXYD4//6jh4IDcYD4//6jh4IDhYD4//6jh4IDmYD4//6jh4IDrYD4',
  '//6jh4IDwYD4//6jh4ID1YD4//6jh4ID6YD4//6jmIED6ACxAQAOEdgAGAAwP/QMAAAA/ulXAKOHggP9gPj//qOHggQRgPj//qOHggQlgPj//qOHggQ5gPj/',
  '/qOHggRNgPj//qOHggRhgPj//qOHggR1gPj//qOHggSJgPj//qOHggSdgPj//qOHggSxgPj//qOHggTFgPj//qOHggTZgPj//qOHggTtgPj//qOHggUBgPj/',
  '/qOHggUVgPj//qOHggUpgPj//qOHggU9gPj//qOHggVRgPj//qOHggVlgPj//qOHggV5gPj//qOHggWNgPj//qOHggWhgPj//qOHggW1gPj//qOHggXJgPj/',
  '/qOHggXdgPj//qOHggXxgPj//qOHggYFgPj//qOHggYZgPj//qOHggYtgPj//qOHggZBgPj//qOHggZVgPj//qOHggZpgPj//qOHggZ9gPj//qOHggaRgPj/',
  '/qOHggalgPj//qOHgga5gPj//qOHggbNgPj//qOHggbhgPj//qOHggb1gPj//qOHggcJgPj//qOHggcdgPj//qOHggcxgPj//qOHggdFgPj//qOHggdZgPj/',
  '/qOHggdtgPj//qOHggeBgPj//qOHggeVgPj//qOHggepgPj//qOHgge9gPj//qOHggfRgPj//qOYgQfQALEBAA4R2AAYADA/9AwAAAD+6VcAo4eCB+WA+P/+',
  'o4eCB/mA+P/+o4eCCA2A+P/+o4eCCCGA+P/+o4eCCDWA+P/+o4eCCEmA+P/+o4eCCF2A+P/+o4eCCHGA+P/+o4eCCIWA+P/+o4eCCJmA+P/+o4eCCK2A+P/+',
  'o4eCCMGA+P/+o4eCCNWA+P/+o4eCCOmA+P/+o4eCCP2A+P/+o4eCCRGA+P/+o4eCCSWA+P/+o4eCCTmA+P/+o4eCCU2A+P/+o4eCCWGA+P/+o4eCCXWA+P/+',
  'o4eCCYmA+P/+o4eCCZ2A+P/+o4eCCbGA+P/+o4eCCcWA+P/+o4eCCdmA+P/+o4eCCe2A+P/+o4eCCgGA+P/+o4eCChWA+P/+o4eCCimA+P/+o4eCCj2A+P/+',
  'o4eCClGA+P/+o4eCCmWA+P/+o4eCCnmA+P/+o4eCCo2A+P/+o4eCCqGA+P/+o4eCCrWA+P/+o4eCCsmA+P/+o4eCCt2A+P/+o4eCCvGA+P/+o4eCCwWA+P/+',
  'o4eCCxmA+P/+o4eCCy2A+P/+o4eCC0GA+P/+o4eCC1WA+P/+o4eCC2mA+P/+o4eCC32A+P/+o4eCC5GA+P/+o4eCC6WA+P/+o4eCC7mA+P/+o5iBC7gAsQEA',
  'DhHYABgAMD/0DAAAAP7pVwCjh4ILzYD4//6jh4IL4YD4//6jh4IL9YD4//6jh4IMCYD4//6jh4IMHYD4//6jh4IMMYD4//6jh4IMRYD4//6jh4IMWYD4//6j',
  'h4IMbYD4//6jh4IMgYD4//6jh4IMlYD4//6jh4IMqYD4//6jh4IMvYD4//6jh4IM0YD4//6jh4IM5YD4//6jh4IM+YD4//6jh4INDYD4//6jh4INIYD4//6j',
  'h4INNYD4//6jh4INSYD4//6jh4INXYD4//6jh4INcYD4//6jh4INhYD4//6jh4INmYD4//6jh4INrYD4//6jh4INwYD4//6jh4IN1YD4//6jh4IN6YD4//6j',
  'h4IN/YD4//6jh4IOEYD4//6jh4IOJYD4//6jh4IOOYD4//6jh4IOTYD4//6jh4IOYYD4//6jh4IOdYD4//6jh4IOiYD4//6jh4IOnYD4//6jh4IOsYD4//6j',
  'h4IOxYD4//6jh4IO2YD4//6jh4IO7YD4//6jh4IPAYD4//6jh4IPFYD4//6jh4IPKYD4//6jh4IPPYD4//6jh4IPUYD4//6jh4IPZYD4//6jh4IPeYD4//6j',
  'h4IPjYD4//6gk6GHgg+hAPj//puBB3WihADN/mAcU7trkbuPs4EAt4r3gQHxggIs8IEM',
  ].join('');

export function keepAwake(): () => void {
  let lock: any = null;
  let dead = false;
  let video: HTMLVideoElement | null = null;
  const native = 'wakeLock' in navigator;

  const acquire = async () => {
    if (document.visibilityState !== 'visible') return;
    try {
      lock = await (navigator as any).wakeLock.request('screen');
      if (dead) lock?.release();
    } catch {
      /* denied, or the battery saver is on */
    }
  };

  // Browsers only start a video after the user has touched the page.
  const play = () => {
    if (dead) return;
    if (!video) {
      video = document.createElement('video');
      video.setAttribute('playsinline', '');
      video.loop = true;
      video.src = CLIP;
      // Off-screen but not hidden: a hidden video does not count as playing.
      video.style.cssText = 'position:fixed;left:-10px;top:-10px;width:1px;height:1px;opacity:0.01;pointer-events:none';
      document.body.appendChild(video);
    }
    video.play().catch(() => {
      /* will be retried on the next touch */
    });
  };

  if (native) {
    acquire();
    document.addEventListener('visibilitychange', acquire);
  } else {
    document.addEventListener('touchend', play);
    document.addEventListener('click', play);
    document.addEventListener('visibilitychange', play);
  }

  return () => {
    dead = true;
    document.removeEventListener('visibilitychange', acquire);
    document.removeEventListener('visibilitychange', play);
    document.removeEventListener('touchend', play);
    document.removeEventListener('click', play);
    lock?.release?.();
    video?.remove();
  };
}

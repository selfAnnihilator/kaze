The seek-tone MP3 and M4A fixtures are synthetic 12-second, 440 Hz mono tones.
They contain no downloaded music. Generated with FFmpeg:

```
ffmpeg -f lavfi -i 'sine=frequency=440:sample_rate=8000:duration=12' -c:a libmp3lame -b:a 16k -write_xing 1 seek-tone.mp3
ffmpeg -f lavfi -i 'sine=frequency=440:sample_rate=8000:duration=12' -c:a aac -b:a 16k -movflags +faststart seek-tone.m4a
```

Progressive tests initially expose only three quarters of each file and append
the tail after a forward seek buffers. They check forward/backward decoding
without a Completed state. MP3 assertions allow decoder preroll after a seek.

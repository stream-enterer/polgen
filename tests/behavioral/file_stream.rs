use eaglemode_rs::emCore::emFileStream::emFileStream;

fn tmp_path(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("eaglemode_test_filestream");
    std::fs::create_dir_all(&dir).ok();
    dir.join(name)
}

#[test]
fn open_read_close() {
    let path = tmp_path("open_read_close.bin");
    std::fs::write(&path, b"hello").unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();
    assert!(fs.IsOpen());

    let mut buf = vec![0u8; 5];
    fs.TryRead(&mut buf).unwrap();
    assert_eq!(&buf, b"hello");

    fs.TryClose().unwrap();
    assert!(!fs.IsOpen());
    std::fs::remove_file(&path).ok();
}

#[test]
fn open_write_close_reread() {
    let path = tmp_path("write_reread.bin");

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "wb").unwrap();
    fs.TryWrite(b"world").unwrap();
    fs.TryClose().unwrap();

    let contents = std::fs::read(&path).unwrap();
    assert_eq!(&contents, b"world");
    std::fs::remove_file(&path).ok();
}

#[test]
fn seek_and_tell() {
    let path = tmp_path("seek_tell.bin");
    std::fs::write(&path, b"abcdefghij").unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();

    assert_eq!(fs.TryTell().unwrap(), 0);
    fs.TrySeek(5).unwrap();
    assert_eq!(fs.TryTell().unwrap(), 5);

    let mut buf = vec![0u8; 3];
    fs.TryRead(&mut buf).unwrap();
    assert_eq!(&buf, b"fgh");
    assert_eq!(fs.TryTell().unwrap(), 8);

    fs.TryClose().unwrap();
    std::fs::remove_file(&path).ok();
}

#[test]
fn read_at_most() {
    let path = tmp_path("read_at_most.bin");
    std::fs::write(&path, b"abc").unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();

    let mut buf = vec![0u8; 10];
    let n = fs.TryReadAtMost(&mut buf).unwrap();
    assert_eq!(n, 3);
    assert_eq!(&buf[..n], b"abc");

    fs.TryClose().unwrap();
    std::fs::remove_file(&path).ok();
}

#[test]
fn read_line() {
    let path = tmp_path("read_line.bin");
    std::fs::write(&path, b"line1\nline2\nline3").unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();

    assert_eq!(fs.TryReadLine(true).unwrap(), "line1");
    assert_eq!(fs.TryReadLine(true).unwrap(), "line2");
    assert_eq!(fs.TryReadLine(true).unwrap(), "line3");

    fs.TryClose().unwrap();
    std::fs::remove_file(&path).ok();
}

#[test]
fn buffered_small_reads() {
    let path = tmp_path("buffered_reads.bin");
    let data: Vec<u8> = (0..=255).collect();
    std::fs::write(&path, &data).unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();

    // Read one byte at a time — should be served from buffer
    for i in 0..=255u8 {
        assert_eq!(fs.TryReadUInt8().unwrap(), i);
    }

    fs.TryClose().unwrap();
    std::fs::remove_file(&path).ok();
}

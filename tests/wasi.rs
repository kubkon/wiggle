use wiggle_runtime::{GuestError, GuestErrorType, GuestPtrMut, GuestString};
use wiggle_test::WasiCtx;

wiggle_generate::from_witx!({
    witx: ["tests/wasi.witx"],
    ctx: WasiCtx,
});

type Result<T> = std::result::Result<T, types::Errno>;

impl GuestErrorType for types::Errno {
    type Context = WasiCtx;
    fn success() -> types::Errno {
        types::Errno::Success
    }
    fn from_error(e: GuestError, ctx: &mut WasiCtx) -> types::Errno {
        eprintln!("GUEST ERROR: {:?}", e);
        ctx.guest_errors.push(e);
        types::Errno::Io
    }
}

impl crate::wasi_snapshot_preview1::WasiSnapshotPreview1 for WasiCtx {
    fn args_get(
        &mut self,
        argv: GuestPtrMut<GuestPtrMut<u8>>,
        argv_buf: GuestPtrMut<u8>,
    ) -> Result<()> {
        unimplemented!("args_get")
    }

    fn args_sizes_get(&mut self) -> Result<(types::Size, types::Size)> {
        unimplemented!("args_sizes_get")
    }

    fn environ_get(
        &mut self,
        environ: GuestPtrMut<GuestPtrMut<u8>>,
        environ_buf: GuestPtrMut<u8>,
    ) -> Result<()> {
        unimplemented!("environ_get")
    }

    fn environ_sizes_get(&mut self) -> Result<(types::Size, types::Size)> {
        unimplemented!("environ_sizes_get")
    }

    fn clock_res_get(&mut self, id: types::Clockid) -> Result<types::Timestamp> {
        unimplemented!("clock_res_get")
    }

    fn clock_time_get(
        &mut self,
        id: types::Clockid,
        precision: types::Timestamp,
    ) -> Result<types::Timestamp> {
        unimplemented!("clock_time_get")
    }

    fn fd_advise(
        &mut self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<()> {
        unimplemented!("fd_advise")
    }

    fn fd_allocate(
        &mut self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
    ) -> Result<()> {
        unimplemented!("fd_allocate")
    }

    fn fd_close(&mut self, fd: types::Fd) -> Result<()> {
        unimplemented!("fd_close")
    }

    fn fd_datasync(&mut self, fd: types::Fd) -> Result<()> {
        unimplemented!("fd_datasync")
    }

    fn fd_fdstat_get(&mut self, fd: types::Fd) -> Result<types::Fdstat> {
        unimplemented!("fd_fdstat_get")
    }

    fn fd_fdstat_set_flags(&mut self, fd: types::Fd, flags: types::Fdflags) -> Result<()> {
        unimplemented!("fd_fdstat_set_flags")
    }

    fn fd_fdstat_set_rights(
        &mut self,
        fd: types::Fd,
        fs_rights_base: types::Rights,
        fs_rights_inherting: types::Rights,
    ) -> Result<()> {
        unimplemented!("fd_fdstat_set_rights")
    }

    fn fd_filestat_get(&mut self, fd: types::Fd) -> Result<types::Filestat> {
        unimplemented!("fd_filestat_get")
    }

    fn fd_filestat_set_size(&mut self, fd: types::Fd, size: types::Filesize) -> Result<()> {
        unimplemented!("fd_filestat_set_size")
    }

    fn fd_filestat_set_times(
        &mut self,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<()> {
        unimplemented!("fd_filestat_set_times")
    }

    fn fd_pread(
        &mut self,
        fd: types::Fd,
        iovs: &types::IovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size> {
        unimplemented!("fd_pread")
    }

    fn fd_prestat_get(&mut self, fd: types::Fd) -> Result<()> {
        unimplemented!("fd_prestat_get")
    }

    fn fd_prestat_dir_name(
        &mut self,
        fd: types::Fd,
        path: GuestPtrMut<u8>,
        path_len: types::Size,
    ) -> Result<()> {
        unimplemented!("fd_prestat_dir_name")
    }

    fn fd_pwrite(
        &mut self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size> {
        unimplemented!("fd_pwrite")
    }

    fn fd_read(&mut self, fd: types::Fd, iovs: &types::IovecArray<'_>) -> Result<types::Size> {
        unimplemented!("fd_read")
    }

    fn fd_readdir(
        &mut self,
        fd: types::Fd,
        buf: GuestPtrMut<u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size> {
        unimplemented!("fd_readdir")
    }

    fn fd_renumber(&mut self, fd: types::Fd, to: types::Fd) -> Result<()> {
        unimplemented!("fd_renumber")
    }

    fn fd_seek(
        &mut self,
        fd: types::Fd,
        offset: types::Filedelta,
        whence: types::Whence,
    ) -> Result<types::Filesize> {
        unimplemented!("fd_seek")
    }

    fn fd_sync(&mut self, fd: types::Fd) -> Result<()> {
        unimplemented!("fd_sync")
    }

    fn fd_tell(&mut self, fd: types::Fd) -> Result<types::Filesize> {
        unimplemented!("fd_tell")
    }

    fn fd_write(&mut self, fd: types::Fd, ciovs: &types::CiovecArray<'_>) -> Result<types::Size> {
        unimplemented!("fd_write")
    }

    fn path_create_directory(&mut self, fd: types::Fd, path: &GuestString<'_>) -> Result<()> {
        unimplemented!("path_create_directory")
    }

    fn path_filestat_get(
        &mut self,
        fd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestString<'_>,
    ) -> Result<types::Filestat> {
        unimplemented!("path_filestat_get")
    }

    fn path_filestat_set_times(
        &mut self,
        fd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestString<'_>,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<()> {
        unimplemented!("path_filestat_set_times")
    }

    fn path_link(
        &mut self,
        old_fd: types::Fd,
        old_flags: types::Lookupflags,
        old_path: &GuestString<'_>,
        new_fd: types::Fd,
        new_path: &GuestString<'_>,
    ) -> Result<()> {
        unimplemented!("path_link")
    }

    fn path_open(
        &mut self,
        fd: types::Fd,
        dirflags: types::Lookupflags,
        path: &GuestString<'_>,
        oflags: types::Oflags,
        fs_rights_base: types::Rights,
        fs_rights_inherting: types::Rights,
        fdflags: types::Fdflags,
    ) -> Result<types::Fd> {
        unimplemented!("path_open")
    }

    fn path_readlink(
        &mut self,
        fd: types::Fd,
        path: &GuestString<'_>,
        buf: GuestPtrMut<'_, u8>,
        buf_len: types::Size,
    ) -> Result<types::Size> {
        unimplemented!("path_readlink")
    }

    fn path_remove_directory(&mut self, fd: types::Fd, path: &GuestString<'_>) -> Result<()> {
        unimplemented!("path_remove_directory")
    }

    fn path_rename(
        &mut self,
        fd: types::Fd,
        old_path: &GuestString<'_>,
        new_fd: types::Fd,
        new_path: &GuestString<'_>,
    ) -> Result<()> {
        unimplemented!("path_rename")
    }

    fn path_symlink(
        &mut self,
        old_path: &GuestString<'_>,
        fd: types::Fd,
        new_path: &GuestString<'_>,
    ) -> Result<()> {
        unimplemented!("path_symlink")
    }

    fn path_unlink_file(&mut self, fd: types::Fd, path: &GuestString<'_>) -> Result<()> {
        unimplemented!("path_unlink_file")
    }

    fn proc_exit(&mut self, rval: types::Exitcode) -> std::result::Result<(), ()> {
        unimplemented!("proc_exit")
    }

    fn proc_raise(&mut self, sig: types::Signal) -> Result<()> {
        unimplemented!("proc_raise")
    }

    fn sched_yield(&mut self) -> Result<()> {
        unimplemented!("sched_yield")
    }

    fn random_get(&mut self, buf: GuestPtrMut<u8>, buf_len: types::Size) -> Result<()> {
        unimplemented!("random_get")
    }

    fn sock_recv(
        &mut self,
        fd: types::Fd,
        ri_data: &types::IovecArray<'_>,
        ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags)> {
        unimplemented!("sock_recv")
    }

    fn sock_send(
        &mut self,
        fd: types::Fd,
        si_data: &types::CiovecArray<'_>,
        si_flags: types::Siflags,
    ) -> Result<types::Size> {
        unimplemented!("sock_send")
    }

    fn sock_shutdown(&mut self, fd: types::Fd, how: types::Sdflags) -> Result<()> {
        unimplemented!("sock_shutdown")
    }
}
